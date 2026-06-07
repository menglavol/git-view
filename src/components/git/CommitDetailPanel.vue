<!--
  提交详情面板（远程 / 本地详情页共用）。

  展示单个提交：元信息（完整 message、作者/提交者、时间、SHA、增删汇总）+ 改动
  文件列表（状态标签 + 增删数）+ 每文件折叠 diff。diff 逐行渲染并按 +/- 着色，
  不使用 v-html（diff 内容来自仓库，避免 XSS）。
-->
<template>
  <!-- loading 兜底为 false：v-loading 不接受 undefined -->
  <div v-loading="loading ?? false" class="commit-detail">
    <div v-if="detail" class="cd-body">
      <!-- 元信息区：标题 + 正文 + 作者/时间/SHA + 增删汇总 -->
      <div class="cd-meta">
        <!-- 提交标题（message 首行） -->
        <div class="cd-summary">{{ firstLine }}</div>
        <!-- 提交正文（首行之后；无正文则不渲染） -->
        <pre v-if="bodyText" class="cd-message">{{ bodyText }}</pre>
        <!-- 短 SHA / 作者 / 时间 / 可选网页跳转 -->
        <div class="cd-fields">
          <span class="sha">{{ detail.shortSha }}</span>
          <span class="author">{{ detail.authorName }}</span>
          <span class="date">{{ formatTime(detail.authoredAt) }}</span>
          <!-- 仅远程提交带 htmlUrl，可跳转平台网页 -->
          <el-button v-if="detail.htmlUrl" link type="primary" size="small" @click="openWeb">
            打开网页
          </el-button>
        </div>
        <!-- 增删汇总 + 文件数 -->
        <div v-if="detail.stats" class="cd-stats">
          <span class="add">+{{ detail.stats.additions }}</span>
          <span class="del">-{{ detail.stats.deletions }}</span>
          <span class="files-count">{{ detail.files.length }} 个文件</span>
        </div>
      </div>

      <!-- 改动文件列表 + 折叠 diff（有文件才渲染折叠面板） -->
      <el-collapse v-if="detail.files.length > 0" class="cd-files">
        <el-collapse-item v-for="f in detail.files" :key="f.path" :name="f.path">
          <!-- 折叠标题：状态标签 + 路径 + 增删数 -->
          <template #title>
            <span class="file-status" :class="`st-${f.status}`">{{ statusLabel(f.status) }}</span>
            <span class="file-path">{{ f.path }}</span>
            <span v-if="f.additions != null" class="add">+{{ f.additions }}</span>
            <span v-if="f.deletions != null" class="del">-{{ f.deletions }}</span>
          </template>
          <!-- diff 逐行着色（不用 v-html，防 XSS） -->
          <div v-if="f.diff" class="diff-text">
            <div
              v-for="(line, i) in splitLines(f.diff)"
              :key="i"
              class="diff-line"
              :class="lineClass(line)"
            >
              {{ line }}
            </div>
          </div>
          <!-- 无 diff：二进制或纯重命名 -->
          <p v-else class="no-diff">无 diff 内容（可能是二进制或纯重命名）</p>
          <!-- 截断提示 -->
          <p v-if="f.truncated" class="truncated">（该文件 diff 过大，已截断）</p>
        </el-collapse-item>
      </el-collapse>
      <!-- 无文件改动（如空提交） -->
      <p v-else class="no-files">本次提交无文件改动</p>
    </div>

    <!-- 未选中提交时的占位 -->
    <el-empty v-else-if="!loading" description="选择一条提交查看详情" />
  </div>
</template>

<script setup lang="ts">
/**
 * CommitDetailPanel 脚本。
 * 纯展示组件：只接收一个 CommitDetail，自身不发请求；message 拆首行/正文，
 * diff 逐行计算着色类。远程与本地详情页复用本组件，保证两端展示一致。
 */
import { computed } from 'vue';
import { ElMessage } from 'element-plus';
import { open as openExternal } from '@tauri-apps/plugin-shell';

import type { CommitDetail, CommitFileStatus } from '@/types/git';

const props = defineProps<{
  detail: CommitDetail | null;
  loading?: boolean;
}>();

/** message 首行（标题）。 */
const firstLine = computed(() => props.detail?.message.split('\n')[0] ?? '');

/** message 正文（首行之后，去掉前导空行与尾部空白）。 */
const bodyText = computed(() => {
  const msg = props.detail?.message ?? '';
  const idx = msg.indexOf('\n');
  if (idx < 0) return '';
  return msg
    .slice(idx + 1)
    .replace(/^\n+/, '')
    .trimEnd();
});

/** 文件状态 → 中文标签。 */
function statusLabel(s: CommitFileStatus): string {
  switch (s) {
    case 'added':
      return '新增';
    case 'deleted':
      return '删除';
    case 'renamed':
      return '重命名';
    default:
      return '修改';
  }
}

/** diff 文本按行拆分，供逐行着色。 */
function splitLines(diff: string): string[] {
  return diff.split('\n');
}

/** 单行 diff 的着色类：+ 新增、- 删除、@@ hunk 头、其余正常。 */
function lineClass(line: string): string {
  if (line.startsWith('+') && !line.startsWith('+++')) return 'dl-add';
  if (line.startsWith('-') && !line.startsWith('---')) return 'dl-del';
  if (line.startsWith('@@')) return 'dl-hunk';
  return '';
}

/** ISO 时间格式化为本地可读字符串。 */
function formatTime(iso: string): string {
  try {
    return new Date(iso).toLocaleString();
  } catch {
    return iso;
  }
}

/** 用系统浏览器打开提交网页（仅远程提交有 htmlUrl）。 */
function openWeb(): void {
  const url = props.detail?.htmlUrl;
  if (!url) return;
  void openExternal(url).catch((e) => {
    ElMessage.error(`打开网页失败：${e instanceof Error ? e.message : String(e)}`);
  });
}
</script>

<style scoped>
/* 面板容器:占满父高并自滚动 */
.commit-detail {
  height: 100%; /* 占满父高 */
  overflow: auto; /* 内容超出时自滚动 */
}

/* 元信息区:底部分割线与文件列表区隔开 */
.cd-meta {
  padding: 4px 4px 12px; /* 下方留白多一点，与分割线呼吸 */
  border-bottom: 1px solid var(--el-border-color-lighter); /* 与文件列表分隔 */
  margin-bottom: 8px; /* 与下方文件区间距 */
}
/* 提交标题:加粗突出 */
.cd-summary {
  font-weight: 600; /* 加粗标题 */
  font-size: 14px; /* 略大字号 */
  margin-bottom: 6px; /* 与正文间距 */
}
/* 提交正文:等宽 + 保留换行 + 次色背景块 */
.cd-message {
  font-family: var(--el-font-family-mono, monospace); /* 等宽便于对齐 */
  font-size: 12px; /* 小字 */
  white-space: pre-wrap; /* 保留换行并自动折行 */
  word-break: break-word; /* 超长词换行 */
  color: var(--el-text-color-regular); /* 常规文字色 */
  background: var(--el-fill-color-light); /* 浅背景区分正文块 */
  padding: 8px; /* 内边距 */
  border-radius: 4px; /* 圆角 */
  margin: 0 0 8px; /* 仅留下间距 */
}
/* 元信息字段行:横排次色小字，允许换行 */
.cd-fields {
  display: flex; /* 横排 */
  align-items: center; /* 垂直居中 */
  gap: 12px; /* 字段间距 */
  font-size: 12px; /* 小字 */
  color: var(--el-text-color-secondary); /* 次色 */
  flex-wrap: wrap; /* 窄抽屉下换行 */
}
/* 短 SHA:等宽 + 主色 */
.cd-fields .sha {
  font-family: var(--el-font-family-mono, monospace); /* 等宽 */
  color: var(--el-color-primary); /* 主色突出 */
}
/* 增删汇总行 */
.cd-stats {
  display: flex; /* 横排 */
  gap: 12px; /* 间距 */
  font-size: 12px; /* 小字 */
  margin-top: 6px; /* 与上方字段行间距 */
}
/* 新增数:绿色 */
.add {
  color: var(--el-color-success); /* 成功绿 */
}
/* 删除数:红色 */
.del {
  color: var(--el-color-danger); /* 危险红 */
}
/* 文件数:次色 */
.files-count {
  color: var(--el-text-color-secondary); /* 次色 */
}

/* 文件折叠标题里的状态标签:小号彩色徽标 */
.file-status {
  font-size: 11px; /* 徽标小字 */
  padding: 0 6px; /* 横向内边距 */
  border-radius: 3px; /* 圆角 */
  margin-right: 8px; /* 与路径间距 */
  color: #fff; /* 白字配彩底 */
}
/* 新增=绿底 */
.st-added {
  background: var(--el-color-success);
}
/* 修改=橙底 */
.st-modified {
  background: var(--el-color-warning);
}
/* 删除=红底 */
.st-deleted {
  background: var(--el-color-danger);
}
/* 重命名=蓝底 */
.st-renamed {
  background: var(--el-color-info);
}
/* 文件路径:占据剩余宽度，等宽截断 */
.file-path {
  flex: 1; /* 占满中间 */
  font-family: var(--el-font-family-mono, monospace); /* 等宽 */
  font-size: 12px; /* 小字 */
  overflow: hidden; /* 溢出隐藏 */
  text-overflow: ellipsis; /* 省略号 */
  white-space: nowrap; /* 不换行 */
  margin-right: 8px; /* 与增删数间距 */
}

/* diff 文本:等宽小字，横向可滚动 */
.diff-text {
  font-family: var(--el-font-family-mono, monospace); /* 等宽 */
  font-size: 12px; /* 小字 */
  overflow-x: auto; /* 长行横向滚动 */
}
/* 单行 diff：保留空白、紧凑行高 */
.diff-line {
  white-space: pre; /* 保留缩进与空白 */
  padding: 0 4px; /* 行内边距 */
  line-height: 1.5; /* 行高 */
}
/* 新增行:绿底绿字 */
.dl-add {
  background: var(--el-color-success-light-9); /* 浅绿底 */
  color: var(--el-color-success); /* 绿字 */
}
/* 删除行:红底红字 */
.dl-del {
  background: var(--el-color-danger-light-9); /* 浅红底 */
  color: var(--el-color-danger); /* 红字 */
}
/* hunk 头(@@ 行):主色 */
.dl-hunk {
  color: var(--el-color-primary); /* 主色 */
}

/* 无 diff / 无文件 / 截断 提示:次色小字 */
.no-diff,
.no-files,
.truncated {
  color: var(--el-text-color-placeholder); /* 占位色 */
  font-size: 12px; /* 小字 */
  padding: 8px; /* 留白 */
  margin: 0; /* 重置 p 默认 margin */
}
</style>
