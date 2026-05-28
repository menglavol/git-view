<!--
  提交历史组件（T087 — US5 V1 简版）。

  V1 范围（与 spec 设计文档 §11.9 一致）：
    - 简单列表展示 commit 摘要、作者、时间
    - 分页加载(滚动到底部 -> 下一页)
    - 暂不实现 graph 可视化、引用气泡等高级功能

  使用：
    <CommitHistory :repo-id="repoId" />
    本组件自管理 commits 列表与分页状态,通过 props.repoId 调用 gitApi.log。
-->
<template>
  <div ref="containerRef" class="commit-history" @scroll="onScroll">
    <ul v-if="commits.length > 0" class="commit-list">
      <li v-for="c in commits" :key="c.sha" class="commit-row" :title="c.message">
        <span class="sha">{{ c.shortSha }}</span>
        <span class="summary">{{ c.summary }}</span>
        <span class="author">{{ c.authorName }}</span>
        <span class="date">{{ formatDate(c.authoredAt) }}</span>
      </li>
    </ul>

    <!-- 空状态:首次加载完成且无提交 -->
    <p v-if="!loading && commits.length === 0" class="empty">无提交历史</p>

    <!-- 加载更多状态指示器,在列表底部出现 -->
    <p v-if="loading" class="loading">加载中...</p>
    <p v-else-if="reachedEnd && commits.length > 0" class="reached-end">— 已加载全部 —</p>
  </div>
</template>

<script setup lang="ts">
/**
 * CommitHistory 脚本（T087）。
 * 简化分页:监听容器 scroll,接近底部自动拉下一页;不做 IntersectionObserver。
 */
import { onMounted, ref, watch } from 'vue';

import { gitApi } from '@/api/git.api';
import type { CommitInfo } from '@/types/git';

/** 父组件传入仓库 id,变化时重置 list 重新加载第一页。 */
const props = defineProps<{ repoId: string }>();

/** 已加载的 commits 列表(append 模式) */
const commits = ref<CommitInfo[]>([]);
/** 当前已加载的页码,从 0 起 */
const page = ref(0);
/** 单页大小,与后端默认对齐 */
const pageSize = 50;
/** 加载中标志,防并发 */
const loading = ref(false);
/** 是否已到底(上一页返回数量 < pageSize) */
const reachedEnd = ref(false);
/** 容器 ref:用于 scroll 监听 */
const containerRef = ref<HTMLElement | null>(null);

/** 加载下一页;reachedEnd 时立即返回。 */
async function loadNextPage(): Promise<void> {
  if (loading.value || reachedEnd.value || !props.repoId) return;
  loading.value = true;
  try {
    const list = await gitApi.log(props.repoId, page.value, pageSize);
    commits.value.push(...list);
    if (list.length < pageSize) reachedEnd.value = true;
    page.value += 1;
  } catch (e) {
    // 静默失败:不打扰用户,后续可由父组件加错误提示
    // eslint-disable-next-line no-console
    console.error('git_log 加载失败:', e);
  } finally {
    loading.value = false;
  }
}

/** 容器滚动监听:距底部 < 80px 时触发加载下一页。 */
function onScroll(): void {
  const el = containerRef.value;
  if (!el) return;
  const distanceToBottom = el.scrollHeight - el.scrollTop - el.clientHeight;
  if (distanceToBottom < 80) {
    void loadNextPage();
  }
}

/** 切换仓库时重置分页与已加载列表。 */
function reset(): void {
  commits.value = [];
  page.value = 0;
  reachedEnd.value = false;
}

// repoId 变化:重置后重新加载;mounted 时触发首次加载
watch(
  () => props.repoId,
  () => {
    reset();
    void loadNextPage();
  },
);
onMounted(() => {
  void loadNextPage();
});

/** ISO 时间格式化:取日期 + 时分,跨平台一致。 */
function formatDate(iso: string): string {
  try {
    const d = new Date(iso);
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
  } catch {
    return iso;
  }
}
function pad(n: number): string {
  return n.toString().padStart(2, '0');
}
</script>

<style scoped>
/* 容器:占满父栏 + 自滚动 */
.commit-history {
  height: 100%; /* 占满父高 */
  overflow: auto; /* 自动滚动 */
}

/* 列表样式重置 */
.commit-list {
  list-style: none; /* 去掉项目符号 */
  padding: 0; /* 重置 */
  margin: 0; /* 重置 */
}

/* 单行:横排 4 列(sha / summary / author / date) */
.commit-row {
  display: grid; /* Grid 实现固定列布局 */
  grid-template-columns: 64px 1fr 100px 130px; /* sha 短码 / 摘要 / 作者 / 日期 */
  gap: 8px; /* 列间距 */
  padding: 4px 8px; /* 行内边距 */
  font-size: 12px; /* 紧凑字号 */
  border-bottom: 1px solid var(--el-border-color-lighter); /* 分割线 */
  cursor: default; /* 暂不支持点击,用默认光标 */
}
.commit-row:hover {
  background: var(--el-fill-color-light); /* hover 浅背景 */
}

/* sha 短码:等宽字体,主色调突出 */
.sha {
  font-family: var(--el-font-family-mono, monospace); /* 等宽 */
  color: var(--el-color-primary); /* 主色 */
}

/* 提交摘要:截断,完整内容用 title 工具提示 */
.summary {
  white-space: nowrap; /* 不换行 */
  overflow: hidden; /* 溢出隐藏 */
  text-overflow: ellipsis; /* 末尾省略号 */
}

/* 作者列:次色 */
.author {
  color: var(--el-text-color-secondary); /* 次色 */
}

/* 日期列:次色 + 等宽便于对齐 */
.date {
  color: var(--el-text-color-placeholder); /* 占位色 */
  font-family: var(--el-font-family-mono, monospace); /* 等宽对齐 */
}

/* 空状态/加载/到底文案:统一居中辅助色 */
.empty,
.loading,
.reached-end {
  text-align: center; /* 居中 */
  color: var(--el-text-color-placeholder); /* 占位色 */
  font-size: 12px; /* 小字 */
  padding: 12px; /* 上下留白 */
  margin: 0; /* 重置 p 默认 margin */
}
</style>
