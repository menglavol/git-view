<!--
  LocalRepoTable.vue（US4 / 目录树形表格）：本地仓库列表组件。
  职责拆分：
    - 仅做展示与事件透传，所有业务逻辑（API 调用、确认弹窗等）由父页面持有
    - 组件内把扁平的 items 转成目录森林（见 repoTree.ts），用 el-table 树形渲染
  树形说明：
    - 目录是可展开/折叠的父节点，仓库是叶子；按 localPath 的文件系统层级组织
    - 目录行：名称列只显示目录名，「子仓库」列显示其子树仓库总数，其余列留空
    - 仓库行：维持原有的名称/路径/分支/状态/最近检查/操作列
    - 若某目录本身是仓库、其下又有子仓库，则合并为一个仓库节点（可展开），
      该行按仓库展示，其下属子仓库数在「子仓库」列显示，勾选时级联其子树
    - 「子仓库」列：目录=子树仓库总数，合并节点=子仓库数，普通仓库无下属显示 —
    - 勾选目录会级联选中其下所有仓库（el-table 树形 selection 不自动级联，手动处理）
    - 对外 update:selection 仍只抛 LocalRepository[]（目录节点被过滤），父页面零改动
-->
<template>
  <!--
    表格配置：
    - row-key 用节点 id：树形渲染与选择保持都依赖每行唯一键
    - tree-props.children 指定子节点字段，目录据此渲染展开箭头
    - 默认折叠（不设 default-expand-all）：初始只显示顶层根目录，点击逐层展开
    - select / select-all / selection-change 三事件配合实现目录勾选的子树级联
  -->
  <ElTable
    ref="tableRef"
    v-loading="loading ?? false"
    :data="treeData"
    style="width: 100%"
    row-key="id"
    :tree-props="{ children: 'children' }"
    :empty-text="emptyText"
    @select="onSelect"
    @select-all="onSelectAll"
    @selection-change="onSelectionChange"
    @expand-change="onExpandChange"
  >
    <!-- 选择框列：用于父页面批量操作；勾选目录/含子仓库的节点会级联其子树 -->
    <ElTableColumn type="selection" width="44" />

    <!-- 名称列：固定宽度，超长省略 + hover 显示完整；只显示纯名称（数量见「子仓库」列） -->
    <ElTableColumn label="仓库名" width="260" show-overflow-tooltip>
      <template #default="{ row }">
        <span v-if="row.type === 'dir'" class="dir-name">{{ row.label }}</span>
        <span v-else class="repo-name" @click="emit('open-detail', row.repo)">{{ row.label }}</span>
      </template>
    </ElTableColumn>

    <!-- 子仓库列：目录=子树仓库总数，合并节点=子仓库数，普通仓库无下属显示 — -->
    <ElTableColumn label="子仓库" width="90" align="center">
      <template #default="{ row }">
        <span v-if="row.type === 'dir'">{{ row.repoCount }}</span>
        <span v-else-if="row.childRepoCount">{{ row.childRepoCount }}</span>
        <span v-else class="muted">—</span>
      </template>
    </ElTableColumn>

    <!-- 本地路径列：固定宽度，超长省略 + hover 显示完整路径 -->
    <ElTableColumn label="本地路径" width="340" show-overflow-tooltip>
      <template #default="{ row }">
        <span class="repo-path">
          {{ row.type === 'dir' ? row.fullPath : row.repo.localPath }}
        </span>
      </template>
    </ElTableColumn>

    <!-- 当前分支列：目录留空，仓库展示分支 -->
    <ElTableColumn label="当前分支" width="140">
      <template #default="{ row }">
        <template v-if="row.type === 'repo'">{{ row.repo.currentBranch ?? '—' }}</template>
        <span v-else class="muted">—</span>
      </template>
    </ElTableColumn>

    <!-- 状态列：目录展示各非零状态计数汇总，仓库展示单个状态标签 -->
    <ElTableColumn label="状态" min-width="170">
      <template #default="{ row }">
        <template v-if="row.type === 'repo'">
          <ElTag :type="statusTag(row.repo.status)" size="small">
            {{ statusLabel(row.repo.status) }}
          </ElTag>
        </template>
        <!-- 目录行：仅展示计数大于 0 的状态，避免堆砌全部 6 个标签 -->
        <span v-else class="summary">
          <ElTag
            v-for="part in summaryParts(row.summary)"
            :key="part.status"
            :type="statusTag(part.status)"
            size="small"
            class="summary-tag"
          >
            {{ statusLabel(part.status) }} {{ part.count }}
          </ElTag>
        </span>
      </template>
    </ElTableColumn>

    <!-- 协议列：仅仓库行，从 remoteUrl 派生 SSH/HTTPS，并提供「切换」入口 -->
    <ElTableColumn label="协议" width="150" align="center">
      <template #default="{ row }">
        <template v-if="row.type === 'repo' && protocolOf(row.repo)">
          <ElTag size="small" :type="protocolOf(row.repo) === 'ssh' ? 'success' : 'info'">
            {{ protocolOf(row.repo) === 'ssh' ? 'SSH' : 'HTTPS' }}
          </ElTag>
          <ElButton
            link
            size="small"
            @click="
              emit('switch-protocol', {
                repo: row.repo,
                target: protocolOf(row.repo) === 'ssh' ? 'https' : 'ssh',
              })
            "
          >
            切换
          </ElButton>
        </template>
        <span v-else class="muted">—</span>
      </template>
    </ElTableColumn>

    <!-- 最近检查列：目录留空，仓库展示时间 -->
    <ElTableColumn label="最近检查" width="160">
      <template #default="{ row }">
        <template v-if="row.type === 'repo'">{{ formatTime(row.repo.lastCheckedAt) }}</template>
        <span v-else class="muted">—</span>
      </template>
    </ElTableColumn>

    <!-- 操作列：仅仓库行有按钮，目录行留空 -->
    <ElTableColumn label="操作" width="320" fixed="right">
      <template #default="{ row }">
        <ElButtonGroup v-if="row.type === 'repo'">
          <ElButton size="small" @click="emit('refresh', row.repo)">刷新</ElButton>
          <ElButton size="small" @click="emit('fetch', row.repo)">Fetch</ElButton>
          <ElButton size="small" @click="emit('open-folder', row.repo)">目录</ElButton>
          <ElButton size="small" @click="emit('open-terminal', row.repo)">终端</ElButton>
          <ElButton size="small" type="danger" plain @click="emit('remove', row.repo)"
            >移除</ElButton
          >
        </ElButtonGroup>
      </template>
    </ElTableColumn>
  </ElTable>
</template>

<script setup lang="ts">
// =====================================================================
// 本地仓库表组件脚本。
// 仅做展示与事件透传：所有业务逻辑（API 调用、确认弹窗等）由父页面持有。
// 树形数据由 repoTree.ts 的 buildRepoForest 生成；选择级联在本组件内手动处理，
// 但对外 emit 的 selection 仍只含真实仓库（LocalRepository），父页面无需改动。
// 选择级联数据流：勾选目录（或含子仓库的仓库节点）→ onSelect 级联子树 →
// selection-change 汇总 → 过滤出仓库后 emit；目录节点永不进入对外 selection。
// =====================================================================
import { computed, nextTick, ref, watch } from 'vue';
import type { TableInstance } from 'element-plus';

import type { LocalRepository, RepositoryStatus } from '@/types/repository';
// 树构建纯函数与节点类型：把扁平仓库列表组织成目录森林
import { buildRepoForest, summaryParts, type RepoLeaf, type RepoTreeNode } from './repoTree';

// 组件 props：扁平仓库数组 + loading 标志 + 空态文案
const props = defineProps<{
  /** 当前要展示的本地仓库数组（由父页面 store 提供，组件内部转成树） */
  items: LocalRepository[];
  /** 加载中标志：true 时表格遮罩并禁用交互 */
  loading?: boolean;
  /** 空态文案：列表为空时显示的提示文字 */
  emptyText?: string;
  /** 受控展开的目录 id 列表（用于「从详情返回」时恢复展开状态） */
  expandedKeys?: string[];
}>();

// 事件签名：父组件通过这些事件接管所有副作用（参数始终是真实 LocalRepository）
const emit = defineEmits<{
  /** 选择行变化：父组件用于批量按钮 disable 判定 */
  (e: 'update:selection', repos: LocalRepository[]): void;
  /** 仓库名点击：跳转到详情页 */
  (e: 'open-detail', repo: LocalRepository): void;
  /** 单仓库刷新：触发 git status 重新计算 */
  (e: 'refresh', repo: LocalRepository): void;
  /** 单仓库 Fetch：等价于在仓库目录执行 git fetch */
  (e: 'fetch', repo: LocalRepository): void;
  /** 在系统文件管理器中打开仓库目录 */
  (e: 'open-folder', repo: LocalRepository): void;
  /** 在系统终端中打开仓库目录 */
  (e: 'open-terminal', repo: LocalRepository): void;
  /** 从列表移除：父页面须做二次确认 */
  (e: 'remove', repo: LocalRepository): void;
  /** 切换该仓库 origin 协议：target 为目标协议（https/ssh） */
  (e: 'switch-protocol', payload: { repo: LocalRepository; target: 'https' | 'ssh' }): void;
  /** 展开的目录 id 变化：父页面同步到 store，以便返回时恢复 */
  (e: 'update:expandedKeys', keys: string[]): void;
}>();

// el-table 实例引用：用于树形选择的程序化级联（toggleRowSelection）
const tableRef = ref<TableInstance>();

// 把扁平仓库列表转成目录森林（可能多棵树，如跨盘符）；
// items 变化时 computed 自动重建，无需手动同步
const treeData = computed(() => buildRepoForest(props.items));

/** 取节点的子节点列表：目录必有 children；合并的仓库节点可选携带 children。 */
function childrenOf(node: RepoTreeNode): RepoTreeNode[] {
  return node.type === 'dir' ? node.children : (node.children ?? []);
}

/** 在树里按 id 查找节点（恢复展开时用于定位行对象）。 */
function findNodeById(nodes: RepoTreeNode[], id: string): RepoTreeNode | null {
  for (const n of nodes) {
    if (n.id === id) return n;
    const found = findNodeById(childrenOf(n), id);
    if (found) return found;
  }
  return null;
}

/** 用户展开/折叠目录时，把最新展开的目录 id 同步给父页面（写入 store）。 */
function onExpandChange(row: RepoTreeNode, expanded: boolean): void {
  if (row.type !== 'dir') return;
  const cur = props.expandedKeys ?? [];
  // 用 Set 去重，避免恢复时 toggle 再次触发导致重复 key
  const next = expanded ? [...new Set([...cur, row.id])] : cur.filter((k) => k !== row.id);
  emit('update:expandedKeys', next);
}

/** 数据就绪后，把 store 记录的展开 keys 恢复到表格（仅「返回」场景非空）。 */
async function restoreExpanded(): Promise<void> {
  const keys = props.expandedKeys ?? [];
  if (keys.length === 0) return;
  // 等 el-table 渲染出新一批行后再逐个展开（toggleRowExpansion 只设状态，渲染时层层展开）
  await nextTick();
  for (const key of keys) {
    const node = findNodeById(treeData.value, key);
    if (node) tableRef.value?.toggleRowExpansion(node, true);
  }
}

// items 变化（如 fetchAll 完成）会重建树，重建后重新套用展开状态
watch(
  () => props.items,
  () => void restoreExpanded(),
);

/** 递归收集某节点子树下的所有节点（子目录、子仓库），用于勾选级联。 */
function collectDescendants(node: RepoTreeNode): RepoTreeNode[] {
  const acc: RepoTreeNode[] = [];
  for (const child of childrenOf(node)) {
    acc.push(child);
    // 继续向下收集，确保深层仓库也被级联
    acc.push(...collectDescendants(child));
  }
  return acc;
}

/**
 * 单行勾选事件：若该行有子节点（目录、或含子仓库的仓库节点），
 * 则把整棵子树同步成相同选中态。el-table 树形 selection 不会自动父子联动。
 */
function onSelect(selection: RepoTreeNode[], row: RepoTreeNode): void {
  const kids = childrenOf(row);
  // 无子节点（普通仓库叶子）：无需级联
  if (kids.length === 0) return;
  // 通过引用相等判断本次操作是勾选还是取消
  const checked = selection.includes(row);
  for (const node of collectDescendants(row)) {
    // 第二参强制目标态，避免 toggle 语义在已选/未选间产生歧义
    tableRef.value?.toggleRowSelection(node, checked);
  }
  // 注：每次 toggleRowSelection 会再触发 selection-change，多次 emit 但最终态一致
}

/** 表头全选：el-table 已对所有可见行（含目录与仓库）递归切换，无需额外级联。 */
function onSelectAll(): void {
  // 故意留空——保留事件绑定以表达「全选交由 el-table 处理」的意图，行为由手动验证兜底
}

/** 选中集合变化：仅把仓库节点映射回真实 repo，目录节点被过滤后 emit 给父页面。 */
function onSelectionChange(rows: RepoTreeNode[]): void {
  // 合并节点 type 也是 'repo'，会被正确纳入；每个仓库唯一对应一行，无重复
  const repos = rows.filter((r): r is RepoLeaf => r.type === 'repo').map((r) => r.repo);
  // 父页面收到的仍是 LocalRepository[]，批量逻辑零感知
  emit('update:selection', repos);
}

/**
 * 6 种 RepositoryStatus 映射到 el-tag 类型颜色。
 * 与 RepoStatusOverview 保持一致；语义说明详见该组件。
 */
function statusTag(s: RepositoryStatus): 'success' | 'warning' | 'info' | 'danger' {
  switch (s) {
    // 干净：绿色
    case 'clean':
      return 'success';
    // 有变更：黄色提示
    case 'dirty':
      return 'warning';
    // 待推送 / 待拉取：蓝色中性
    case 'ahead':
    case 'behind':
      return 'info';
    // 已分叉：红色高优先级
    case 'diverged':
      return 'danger';
    // 未知：默认蓝色
    case 'unknown':
    default:
      return 'info';
  }
}

/** 状态枚举到中文标签的映射；UI 改文案需同步 RepoStatusOverview。 */
function statusLabel(s: RepositoryStatus): string {
  switch (s) {
    // 工作区干净，无未提交改动
    case 'clean':
      return '干净';
    // 存在未提交改动
    case 'dirty':
      return '有变更';
    // 本地领先远端，待推送
    case 'ahead':
      return '待推送';
    // 本地落后远端，待拉取
    case 'behind':
      return '待拉取';
    // 与远端分叉，需手动处理
    case 'diverged':
      return '已分叉';
    // 状态未知或尚未检查
    case 'unknown':
    default:
      return '未知';
  }
}

/** ISO 时间戳格式化为本地短日期；解析失败时原样返回。 */
function formatTime(iso?: string): string {
  // 空值或 undefined：返回占位符避免渲染空白
  if (!iso) return '—';
  try {
    // 使用浏览器系统时区，与用户认知一致
    return new Date(iso).toLocaleString();
  } catch {
    // 解析失败的兜底：直接展示原始字符串
    return iso;
  }
}

/** 从本地仓库的 remoteUrl 派生当前协议；无远程或无法识别返回 null。 */
function protocolOf(repo: LocalRepository): 'https' | 'ssh' | null {
  // 空 remoteUrl（未关联远程）视为无协议，列内显示占位符
  const u = repo.remoteUrl?.trim();
  if (!u) return null;
  // http/https → HTTPS 类；git@ 与 ssh:// → SSH 类
  if (u.startsWith('https://') || u.startsWith('http://')) return 'https';
  if (u.startsWith('git@') || u.startsWith('ssh://')) return 'ssh';
  // 其余（本地路径等）无法判定协议
  return null;
}
</script>

<style scoped>
/* 以下样式用于区分目录行与仓库行，并复用原有仓库样式 */

/* 目录名样式：加粗以与仓库行区分 */
.dir-name {
  font-weight: 600;
}

/* 目录状态汇总：多个小标签横向排列并允许换行 */
.summary {
  display: inline-flex;
  flex-wrap: wrap;
  gap: 4px;
}

/* 单个汇总标签：保证与文字基线对齐 */
.summary-tag {
  vertical-align: middle;
}

/* 空值占位（无下属仓库、目录行的分支/最近检查列等）：弱化显示 */
.muted {
  color: var(--el-text-color-placeholder);
}

/* 仓库名样式：主色 + 指针光标，提示可点击 */
.repo-name {
  color: var(--el-color-primary);
  cursor: pointer;
  font-weight: 500;
}

/* 仓库名 hover 下划线，进一步强化可点击 */
.repo-name:hover {
  text-decoration: underline;
}

/* 本地路径样式：次要颜色 + 等宽字体便于扫读 */
.repo-path {
  color: var(--el-text-color-secondary);
  font-size: 12px;
  font-family: var(--el-font-family-mono, monospace);
}
</style>
