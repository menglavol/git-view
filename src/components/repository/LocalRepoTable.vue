<!--
  LocalRepoTable.vue（T073 / US4）：本地仓库列表表组件。
  职责拆分：
    - 仅做展示与事件透传，所有业务逻辑（API 调用、确认弹窗等）由父页面持有
    - 抽取为独立组件方便单元测试与未来虚拟滚动改造
  列设计：
    - 选择框：用于父页面批量操作（批量 Fetch / 批量移除）
    - 仓库名：从 localPath 取末段，点击 emit open-detail（详情页 US5 实现）
    - 本地路径：超长省略 + tooltip，使用等宽字体
    - 当前分支 / 状态 / 最近检查：只读展示
    - 操作列：5 个按钮分别 emit 给父组件处理
-->
<template>
  <ElTable
    v-loading="loading ?? false"
    :data="items"
    style="width: 100%"
    row-key="id"
    :empty-text="emptyText"
    @selection-change="onSelectionChange"
  >
    <!-- 选择框列：宽度固定 44，用于多选 -->
    <ElTableColumn type="selection" width="44" />

    <!-- 仓库名列：点击触发 open-detail 事件 -->
    <ElTableColumn label="仓库名" min-width="180">
      <template #default="{ row }">
        <span class="repo-name" @click="emit('open-detail', row)">
          {{ repoBasename(row.localPath) }}
        </span>
      </template>
    </ElTableColumn>

    <!-- 本地路径列：超长省略并支持 tooltip 浮层 -->
    <ElTableColumn label="本地路径" min-width="240" show-overflow-tooltip>
      <template #default="{ row }">
        <span class="repo-path">{{ row.localPath }}</span>
      </template>
    </ElTableColumn>

    <!-- 当前分支列：未关联分支时显示占位符 -->
    <ElTableColumn label="当前分支" width="140">
      <template #default="{ row }">
        {{ row.currentBranch ?? '—' }}
      </template>
    </ElTableColumn>

    <!-- 状态列：用 el-tag 配合状态色与中文标签 -->
    <ElTableColumn label="状态" width="100">
      <template #default="{ row }">
        <ElTag :type="statusTag(row.status)" size="small">
          {{ statusLabel(row.status) }}
        </ElTag>
      </template>
    </ElTableColumn>

    <!-- 最近检查时间列：ISO 时间戳格式化为本地时间 -->
    <ElTableColumn label="最近检查" width="160">
      <template #default="{ row }">
        {{ formatTime(row.lastCheckedAt) }}
      </template>
    </ElTableColumn>

    <!-- 操作列：固定在右侧，避免横向滚动时丢失 -->
    <ElTableColumn label="操作" width="320" fixed="right">
      <template #default="{ row }">
        <ElButtonGroup>
          <ElButton size="small" @click="emit('refresh', row)">刷新</ElButton>
          <ElButton size="small" @click="emit('fetch', row)">Fetch</ElButton>
          <ElButton size="small" @click="emit('open-folder', row)">目录</ElButton>
          <ElButton size="small" @click="emit('open-terminal', row)">终端</ElButton>
          <ElButton size="small" type="danger" plain @click="emit('remove', row)"> 移除 </ElButton>
        </ElButtonGroup>
      </template>
    </ElTableColumn>
  </ElTable>
</template>

<script setup lang="ts">
// =====================================================================
// 本地仓库表组件脚本。
// 仅做展示与事件透传：所有业务逻辑（API 调用、确认弹窗等）由父页面持有。
// 与 RepoStatusOverview.vue 的关系：
//   - 共享 RepositoryStatus 枚举与 statusTag / statusLabel 映射逻辑
//   - 未来若有更多消费点可抽 utils 集中维护
// =====================================================================
import type { LocalRepository, RepositoryStatus } from '@/types/repository';

// 组件 props：列表数据 + loading 标志 + 空态文案
defineProps<{
  /** 当前要展示的本地仓库数组（由父页面 store 提供） */
  items: LocalRepository[];
  /** 加载中标志：true 时表格遮罩并禁用交互 */
  loading?: boolean;
  /** 空态文案：列表为空时显示的提示文字 */
  emptyText?: string;
}>();

// 事件签名：父组件通过这些事件接管所有副作用
const emit = defineEmits<{
  /** 选择行变化：父组件用于批量按钮 disable 判定 */
  (e: 'update:selection', repos: LocalRepository[]): void;
  /** 仓库名点击：跳转到详情页（US5 实现） */
  (e: 'open-detail', repo: LocalRepository): void;
  /** 单仓库刷新：触发 git status 重新计算 */
  (e: 'refresh', repo: LocalRepository): void;
  /** 单仓库 Fetch：等价于在仓库目录执行 git fetch */
  (e: 'fetch', repo: LocalRepository): void;
  /** 在系统文件管理器中打开仓库目录 */
  (e: 'open-folder', repo: LocalRepository): void;
  /** 在系统终端中打开仓库目录 */
  (e: 'open-terminal', repo: LocalRepository): void;
  /** 从列表移除：父页面须做 ElMessageBox 二次确认 */
  (e: 'remove', repo: LocalRepository): void;
}>();

/** 选择框变更时把当前选中行透传给父组件。 */
function onSelectionChange(rows: LocalRepository[]): void {
  // 仅做事件透传；selection 状态由父组件持有，避免 store 双源
  emit('update:selection', rows);
}

/**
 * 从本地路径提取末段目录名作为仓库名展示。
 * 例：`/Users/a/Projects/git-view/` → `git-view`
 *     `C:\Users\a\foo\bar` → `bar`
 */
function repoBasename(p: string): string {
  // 同时兼容 POSIX 与 Windows 路径分隔符
  const parts = p.split(/[\\/]/).filter(Boolean);
  // filter(Boolean) 去掉空段（处理 trailing slash 情形）
  return parts.length > 0 ? parts[parts.length - 1] : p;
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
    case 'clean':
      return '干净';
    case 'dirty':
      return '有变更';
    case 'ahead':
      return '待推送';
    case 'behind':
      return '待拉取';
    case 'diverged':
      return '已分叉';
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
</script>

<style scoped>
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
