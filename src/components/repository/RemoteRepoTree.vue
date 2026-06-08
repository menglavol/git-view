<template>
  <!--
    远程仓库树形表格：平台→命名空间层级展示。
    - row-key 用节点 id，tree-props.children 指定子节点字段
    - 默认折叠：初始只显示平台根节点，点击逐层展开，避免一次铺满
    - select / selection-change 配合实现目录勾选的子树级联（el-table 不自动级联）
    - 对外 update:selection 仍只抛 RemoteRepository[]（目录节点被过滤），与列表视图一致
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
    @selection-change="onSelectionChange"
  >
    <!-- 选择框列：勾选平台/命名空间目录会级联其子树全部仓库 -->
    <ElTableColumn type="selection" width="44" />

    <!-- 仓库列：只放名称（纯 inline，确保跟随树形缩进）；描述拆到独立列避免缩进冲突 -->
    <ElTableColumn label="仓库" min-width="220" show-overflow-tooltip>
      <template #default="{ row }">
        <!-- 目录/平台节点：展示层级名（压缩后可能是多段拼接，如 group/subgroup） -->
        <span v-if="row.type === 'dir'" class="dir-name" :class="{ platform: row.isPlatform }">
          {{ row.label }}
        </span>
        <!-- 仓库叶子：名称可点击进入详情 -->
        <span v-else class="repo-name" @click="emit('open-detail', row.repo)">{{ row.label }}</span>
      </template>
    </ElTableColumn>

    <!-- 描述列：仓库行展示描述（单行省略，hover 看全）；目录行留空。独立成列以免影响首列缩进 -->
    <ElTableColumn label="描述" min-width="200" show-overflow-tooltip>
      <template #default="{ row }">
        <span v-if="row.type === 'repo' && row.repo.description" class="repo-desc">
          {{ row.repo.description }}
        </span>
        <span v-else class="muted">—</span>
      </template>
    </ElTableColumn>

    <!-- 平台列：仓库行展示来源平台标签，目录行留空（折叠/滚动时便于快速辨认平台） -->
    <ElTableColumn label="平台" width="90">
      <template #default="{ row }">
        <ElTag v-if="row.type === 'repo'" :type="platformTag(row.repo.platform)" size="small">
          {{ platformLabel(row.repo.platform) }}
        </ElTag>
        <span v-else class="muted">—</span>
      </template>
    </ElTableColumn>

    <!-- 仓库数列：目录展示子树仓库总数，仓库行留空 -->
    <ElTableColumn label="仓库数" width="80" align="center">
      <template #default="{ row }">
        <!-- 目录聚合的子树仓库数；仓库叶子本身不计数 -->
        <span v-if="row.type === 'dir'">{{ row.repoCount }}</span>
        <span v-else class="muted">—</span>
      </template>
    </ElTableColumn>

    <!-- 可见性列：仅仓库行展示标签，目录行留空 -->
    <ElTableColumn label="可见性" width="80">
      <template #default="{ row }">
        <!-- 公开/内部/私有，对应绿/蓝/黄三色标签 -->
        <ElTag
          v-if="row.type === 'repo'"
          :type="visTag(row.repo.visibility)"
          size="small"
          effect="plain"
        >
          {{ visLabel(row.repo.visibility) }}
        </ElTag>
        <span v-else class="muted">—</span>
      </template>
    </ElTableColumn>

    <!-- 默认分支列：仅仓库行 -->
    <ElTableColumn label="默认分支" width="120">
      <template #default="{ row }">
        <!-- 仓库的默认分支名（如 main / master） -->
        <template v-if="row.type === 'repo'">{{ row.repo.defaultBranch }}</template>
        <span v-else class="muted">—</span>
      </template>
    </ElTableColumn>

    <!-- 最近推送列：仅仓库行 -->
    <ElTableColumn label="最近推送" width="140">
      <template #default="{ row }">
        <!-- 最近一次 push 时间，按本地时区显示短日期 -->
        <template v-if="row.type === 'repo'">{{ formatTime(row.repo.lastPushedAt) }}</template>
        <span v-else class="muted">—</span>
      </template>
    </ElTableColumn>

    <!-- 操作列：仅仓库行有按钮，目录行留空 -->
    <ElTableColumn label="操作" width="240" fixed="right">
      <template #default="{ row }">
        <ElButtonGroup v-if="row.type === 'repo'">
          <!-- 克隆：打开批量克隆对话框（单仓库即一行可编辑目录名） -->
          <ElButton size="small" @click="emit('clone', row.repo)">Clone</ElButton>
          <!-- 复制 HTTPS 克隆地址到剪贴板 -->
          <ElButton size="small" @click="emit('copy-url', { repo: row.repo, type: 'https' })">
            复制
          </ElButton>
          <!-- 切换收藏：收藏后可用「仅收藏」过滤快速定位 -->
          <ElButton
            size="small"
            :type="row.repo.isFavorite ? 'warning' : 'default'"
            @click="emit('toggle-favorite', row.repo)"
          >
            {{ row.repo.isFavorite ? '已收藏' : '收藏' }}
          </ElButton>
        </ElButtonGroup>
      </template>
    </ElTableColumn>
  </ElTable>
</template>

<script setup lang="ts">
// =====================================================================
// 远程仓库树形表组件。
// 仅做展示与事件透传：业务逻辑（克隆、收藏、复制等）由父页面持有。
// 树形数据由 remoteRepoTree.ts 的 buildRemoteForest 生成；选择级联在本组件内
// 手动处理，但对外 emit 的 selection 仍只含真实仓库（RemoteRepository）。
// =====================================================================
import { computed, ref } from 'vue';
import type { TableInstance } from 'element-plus';

import type { RemoteRepository, Visibility } from '@/types/repository';
import type { GitPlatform } from '@/types/account';
// 树构建纯函数与节点类型：把扁平仓库列表组织成平台→命名空间森林
import { buildRemoteForest, type RemoteRepoLeaf, type RemoteTreeNode } from './remoteRepoTree';

const props = defineProps<{
  /** 当前要展示的远程仓库数组（由父页面 store 提供，组件内部转成树） */
  items: RemoteRepository[];
  /** 加载中标志 */
  loading?: boolean;
  /** 空态文案 */
  emptyText?: string;
}>();

const emit = defineEmits<{
  /** 选择行变化：父组件用于批量按钮 disable 判定（始终是真实仓库） */
  (e: 'update:selection', repos: RemoteRepository[]): void;
  /** 仓库名点击：打开详情 */
  (e: 'open-detail', repo: RemoteRepository): void;
  /** 单仓库克隆 */
  (e: 'clone', repo: RemoteRepository): void;
  /** 切换收藏 */
  (e: 'toggle-favorite', repo: RemoteRepository): void;
  /** 复制克隆地址 */
  (e: 'copy-url', payload: { repo: RemoteRepository; type: 'https' | 'ssh' }): void;
}>();

// el-table 实例引用：用于树形选择的程序化级联（toggleRowSelection）
const tableRef = ref<TableInstance>();

// 扁平列表转平台→命名空间森林；items 变化时 computed 自动重建
const treeData = computed(() => buildRemoteForest(props.items));

/** 取节点子节点：目录有 children，仓库叶子无下属。 */
function childrenOf(node: RemoteTreeNode): RemoteTreeNode[] {
  return node.type === 'dir' ? node.children : [];
}

/** 递归收集子树下全部节点，用于勾选级联。 */
function collectDescendants(node: RemoteTreeNode): RemoteTreeNode[] {
  const acc: RemoteTreeNode[] = [];
  for (const child of childrenOf(node)) {
    // 先收当前子节点，再向下递归，确保深层命名空间里的仓库也被纳入级联
    acc.push(child);
    acc.push(...collectDescendants(child));
  }
  return acc;
}

/** 勾选目录（平台/命名空间）时把整棵子树同步成相同选中态。 */
function onSelect(selection: RemoteTreeNode[], row: RemoteTreeNode): void {
  const kids = childrenOf(row);
  // 仓库叶子无子节点，无需级联
  if (kids.length === 0) return;
  // 引用相等判断本次是勾选还是取消
  const checked = selection.includes(row);
  // 第二参强制目标态，避免 toggle 语义在已选/未选间产生歧义
  for (const node of collectDescendants(row)) {
    tableRef.value?.toggleRowSelection(node, checked);
  }
}

/** 选中集合变化：仅把仓库节点映射回真实 repo，目录节点过滤后 emit。 */
function onSelectionChange(rows: RemoteTreeNode[]): void {
  // 过滤掉平台/命名空间目录节点，只回传真实仓库，父页面批量逻辑零感知树结构
  const repos = rows.filter((r): r is RemoteRepoLeaf => r.type === 'repo').map((r) => r.repo);
  emit('update:selection', repos);
}

/** 平台枚举到展示名。 */
function platformLabel(p: GitPlatform): string {
  return p === 'github' ? 'GitHub' : p === 'gitlab' ? 'GitLab' : 'Gitee';
}

/** 平台到 el-tag 颜色（与列表视图保持一致）。 */
function platformTag(p: GitPlatform): 'info' | 'warning' | 'danger' {
  return p === 'github' ? 'info' : p === 'gitlab' ? 'warning' : 'danger';
}

/** 可见性枚举到中文标签。 */
function visLabel(v: Visibility): string {
  // 三态：公开 / 内部（GitLab 特有） / 私有
  return v === 'public' ? '公开' : v === 'internal' ? '内部' : '私有';
}

/** 可见性到 el-tag 颜色。 */
function visTag(v: Visibility): 'success' | 'info' | 'warning' {
  // 公开=绿、内部=蓝、私有=黄，与列表视图保持一致
  return v === 'public' ? 'success' : v === 'internal' ? 'info' : 'warning';
}

/** ISO 时间戳格式化为本地短日期；空值或解析失败返回占位符。 */
function formatTime(iso?: string): string {
  if (!iso) return '—';
  try {
    return new Date(iso).toLocaleDateString();
  } catch {
    // 非法时间字符串：原样返回，避免抛错中断渲染
    return iso;
  }
}
</script>

<style scoped>
/* 目录名：加粗以区分仓库行 */
.dir-name {
  font-weight: 600;
}

/* 平台根节点：主色高亮，强调顶层分组 */
.dir-name.platform {
  color: var(--el-color-primary);
}

/* 仓库名：主色 + 指针光标，提示可点击进入详情（单行省略由列的 show-overflow-tooltip 处理） */
.repo-name {
  color: var(--el-color-primary);
  cursor: pointer;
  font-weight: 500;
}

.repo-name:hover {
  text-decoration: underline;
}

/* 仓库描述：次要颜色、小字号（独立列，单行省略 + tooltip 由列自带） */
.repo-desc {
  color: var(--el-text-color-secondary);
  font-size: 12px;
}

/* 空值占位（目录行的可见性/分支等列）：弱化显示 */
.muted {
  color: var(--el-text-color-placeholder);
}
</style>
