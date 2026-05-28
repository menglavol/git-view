<!--
  RepoStatusOverview（T074）：单仓库状态总览紧凑卡片。
  使用场景：
    - LocalRepositories.vue：选中单仓库时显示在表下方
    - RepositoryDetail.vue（US5 接入）：作为详情页顶栏概览
    - Dashboard.vue（T107 Phase 10 接入）：作为仪表盘卡片
  设计：
    - 仅展示，不承载操作（操作交给父页面的工具栏 / 表行按钮）
    - 6 种 RepositoryStatus 全部覆盖；状态枚举的中文标签与 Tag 颜色映射
      与 LocalRepoTable 保持一致，避免不同入口语义漂移
-->
<template>
  <div class="repo-status-overview">
    <!-- 状态徽标：放在最显眼位置便于快速识别 -->
    <ElTag :type="statusTag(repo.status)" size="default">
      {{ statusLabel(repo.status) }}
    </ElTag>

    <!-- 当前分支：null 时显示 detached 占位 -->
    <span class="meta">
      <span class="label">分支</span>
      <span class="value">{{ repo.currentBranch ?? '（detached）' }}</span>
    </span>

    <!-- 远程地址：常用于辨识同名仓库的不同 fork -->
    <span v-if="repo.remoteUrl" class="meta">
      <span class="label">远程</span>
      <span class="value remote-url">{{ repo.remoteUrl }}</span>
    </span>

    <!-- 最近一次状态检查时间 -->
    <span class="meta">
      <span class="label">最近检查</span>
      <span class="value">{{ formatTime(repo.lastCheckedAt) }}</span>
    </span>
  </div>
</template>

<script setup lang="ts">
// =====================================================================
// 单仓库状态总览组件脚本（T074）。
// 设计目标：
//   - 把「状态徽标 + 分支 + 远程 + 最近检查时间」组合为一个紧凑信息条
//   - 与 LocalRepoTable 内同样的状态映射逻辑保持一致（防止两处漂移）
//   - 不绑定任何 store；通过 props.repo 接收数据，便于复用到 Dashboard / Detail
// 与同目录 LocalRepoTable.vue 的关系：
//   - 都依赖 RepositoryStatus 枚举（6 种值）
//   - 状态颜色 / 标签映射函数语义相同，未来如要统一可抽 utils
// =====================================================================
import type { LocalRepository, RepositoryStatus } from '@/types/repository';

// 组件 props 仅一个 repo 字段，便于在不同入口直接挂载
defineProps<{
  /** 待展示的本地仓库元数据 */
  repo: LocalRepository;
}>();

/**
 * 状态枚举 → el-tag 类型颜色。
 * 颜色含义：
 *   - success：干净（clean）
 *   - warning：脏（dirty）
 *   - info：待推/拉（ahead/behind）或未知（unknown）
 *   - danger：已分叉（diverged，需用户介入）
 */
function statusTag(s: RepositoryStatus): 'success' | 'warning' | 'info' | 'danger' {
  switch (s) {
    // 干净状态：绿色对应「无需操作」
    case 'clean':
      return 'success';
    // 工作区有变更：黄色提示用户需提交
    case 'dirty':
      return 'warning';
    // 待推送 / 待拉取：蓝色中性提示
    case 'ahead':
    case 'behind':
      return 'info';
    // 分叉：红色高优先级提示
    case 'diverged':
      return 'danger';
    // 未知状态（路径不存在等）：与 ahead/behind 同蓝
    case 'unknown':
    default:
      return 'info';
  }
}

/**
 * 状态枚举 → 中文标签。
 * 与 LocalRepoTable 保持完全一致；UI 改文案需同步两处。
 */
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

/**
 * ISO 时间戳格式化为本地时间字符串。
 * 边界：iso 为 undefined / 空 / 非法字符串时返回占位 '—'。
 */
function formatTime(iso?: string): string {
  if (!iso) return '—';
  try {
    // toLocaleString 使用浏览器系统时区，与用户认知一致
    return new Date(iso).toLocaleString();
  } catch {
    // 解析失败时降级为原文，便于排查
    return iso;
  }
}
</script>

<style scoped>
/* 总览条容器：横向 flex，超长自动换行 */
.repo-status-overview {
  align-items: center;
  background: var(--el-fill-color-light);
  border-radius: 6px;
  display: flex;
  flex-wrap: wrap;
  gap: 16px;
  padding: 10px 14px;
}

/* 单个 meta 块：标签 + 值的横向组合 */
.meta {
  align-items: center;
  display: flex;
  gap: 6px;
  font-size: 13px;
}

/* meta 标签使用次要颜色，弱化视觉权重 */
.meta .label {
  color: var(--el-text-color-secondary);
}

/* meta 值使用常规颜色，作为主要信息 */
.meta .value {
  color: var(--el-text-color-regular);
}

/* 远程地址使用等宽字体，便于扫读 URL */
.meta .remote-url {
  font-family: var(--el-font-family-mono, monospace);
  max-width: 360px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
