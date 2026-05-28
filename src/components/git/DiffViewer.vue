<!--
  Diff 查看器组件（T084 — US5）。

  渲染统一 diff（unified diff）格式：
    - hunk 头部行 (@@...@@): 灰色背景
    - + 行: 绿色背景
    - - 行: 红色背景
    - context 行: 默认背景

  保护策略：
    - 后端 git_reader_service::diff 在 > 1MB 时返回 truncated = true
    - 接收 file 为二进制 / 不存在时由后端返回提示,本组件透传展示
    - 本组件不渲染二进制内容,V1 简化为"文件过大不予显示"占位

  使用：
    <DiffViewer :result="diffResult" />
    父组件根据当前选中文件调用 gitApi.diff 后传入 result。
-->
<template>
  <div class="diff-viewer">
    <!-- 无选中文件时的初始占位 -->
    <div v-if="!result" class="placeholder">
      <p class="hint">在左侧文件变更列表中选择一个文件查看 diff</p>
    </div>

    <!-- 空 diff (无变化或暂存 diff 与工作区 diff 相同) -->
    <div v-else-if="!hasContent" class="placeholder">
      <p class="hint">该文件无差异显示</p>
    </div>

    <!-- 二进制 / 大文件检测占位:后端可能返回特定文本 -->
    <div v-else-if="isLikelyBinary" class="placeholder warning">
      <p>无法显示二进制文件 diff</p>
      <p class="hint">请使用外部工具对比</p>
    </div>

    <!-- 正常 diff 渲染:每行带状态 class -->
    <pre v-else class="diff-content"><span
        v-for="(line, idx) in renderedLines"
        :key="idx"
        :class="['diff-line', line.cls]"
      >{{ line.text }}
</span></pre>

    <!-- 截断提示:超过 1MB 时后端会返回 truncated = true -->
    <p v-if="result?.truncated" class="truncate-hint">
      ⚠ Diff 已被截断 (超过 1 MB 限制),请使用外部工具查看完整差异
    </p>
  </div>
</template>

<script setup lang="ts">
/**
 * DiffViewer 脚本（T084）。
 * 输入:父组件根据当前选中文件调用 gitApi.diff 后传入的 DiffResult。
 * 输出:无 emit,纯展示组件。
 */
import { computed } from 'vue';

import type { DiffResult } from '@/types/git';

/** 仅接受 result 一个 prop,组件保持纯渲染语义 */
const props = defineProps<{ result: DiffResult | null }>();

/** 是否有有效的 diff 内容 */
const hasContent = computed(() => Boolean(props.result?.text?.trim()));

/**
 * 简单二进制检测:diff 中含 "Binary files" 关键字(git diff 对二进制的标记)。
 * 这是 V1 简化策略;更完整的方案需后端在 diff 阶段标记 binary 字段。
 */
const isLikelyBinary = computed(() => {
  const text = props.result?.text ?? '';
  return /^Binary files\s+/m.test(text) || /\bdiffer\b/.test(text.split('\n', 1)[0] ?? '');
});

/** 按行预先分类,模板中直接渲染避免每次重计算。 */
const renderedLines = computed(() => {
  const text = props.result?.text ?? '';
  if (!text) return [];
  // split('\n') 不丢弃空行,末尾如果是 \n 会得到额外空 entry,这里保留以维持视觉对齐
  return text.split('\n').map((line) => ({
    text: line,
    cls: classifyLine(line),
  }));
});

/** 按 diff 行首字符决定渲染类:hunk / add / del / file-header / context。 */
function classifyLine(line: string): string {
  // hunk 头:@@...@@ 标记差异块起止位置
  if (line.startsWith('@@')) return 'hunk';
  // 文件头:--- a/path、+++ b/path、diff --git ... 这些是 git 自身的 metadata
  if (line.startsWith('+++') || line.startsWith('---') || line.startsWith('diff --git')) {
    return 'file-header';
  }
  if (line.startsWith('index ')) return 'file-header';
  // 仅当首字符是单独的 + / - 时表示行级新增/删除（避免与 +++ / --- 冲突）
  if (line.startsWith('+')) return 'add';
  if (line.startsWith('-')) return 'del';
  // 其他视为 context（无前缀或以空格开头）
  return 'context';
}
</script>

<style scoped>
/* 容器:占满父栏,内部 pre 自带滚动 */
.diff-viewer {
  display: flex; /* 纵向布局 */
  flex-direction: column; /* 占位与内容垂直堆叠 */
  height: 100%; /* 撑满父高 */
  min-height: 0; /* 允许 flex 子元素内部滚动 */
}

/* 占位文案:垂直居中,辅助色 */
.placeholder {
  display: flex; /* flex 用于垂直水平居中 */
  flex-direction: column; /* 上下排列两行提示 */
  justify-content: center; /* 垂直居中 */
  align-items: center; /* 水平居中 */
  height: 100%; /* 占满容器 */
  color: var(--el-text-color-secondary); /* 次色 */
  text-align: center; /* 多行也居中 */
  padding: 16px; /* 适当留白 */
}
.placeholder .hint {
  font-size: 13px; /* 二级提示字号 */
  margin-top: 4px; /* 与上一行间距 */
  color: var(--el-text-color-placeholder); /* 更弱的提示色 */
}
/* 警告样态:用于二进制/大文件提示 */
.placeholder.warning {
  color: var(--el-color-warning); /* 警告黄 */
}

/* diff 主体:pre 用等宽字体,自定义滚动 */
.diff-content {
  flex: 1; /* 占满剩余 */
  margin: 0; /* 重置默认 */
  padding: 0; /* 重置 */
  font-family: var(--el-font-family-mono, monospace); /* 等宽字体 */
  font-size: 12px; /* 紧凑字号 */
  line-height: 1.5; /* 行高便于阅读 */
  overflow: auto; /* 自动滚动 */
  background: var(--el-fill-color-blank); /* 与外栏一致 */
  border-radius: 4px; /* 圆角 */
}

/* 单行:占满宽度,带左侧 padding 给状态字符留位 */
.diff-line {
  display: block; /* 整行展示 */
  white-space: pre; /* 保留空格与缩进 */
  padding: 0 8px; /* 左右内边距 */
}

/* 新增行:绿色背景 + 深绿文字 */
.diff-line.add {
  background: rgba(0, 200, 100, 0.12); /* 半透明绿色 */
  color: var(--el-color-success); /* 成功色 */
}

/* 删除行:红色背景 + 深红文字 */
.diff-line.del {
  background: rgba(255, 80, 80, 0.12); /* 半透明红色 */
  color: var(--el-color-danger); /* 危险色 */
}

/* hunk 行:灰色背景标识分块边界 */
.diff-line.hunk {
  background: var(--el-fill-color); /* 灰色分块 */
  color: var(--el-text-color-secondary); /* 次色 */
  font-style: italic; /* 斜体区分 metadata */
}

/* 文件元信息行:辅助色,字号略小 */
.diff-line.file-header {
  color: var(--el-text-color-placeholder); /* 辅助色 */
  font-size: 11px; /* 更小字号 */
}

/* context 行:默认色,无背景 */
.diff-line.context {
  color: var(--el-text-color-regular); /* 正常字色 */
}

/* 截断提示:警示色,放在 diff 下方 */
.truncate-hint {
  margin: 8px 0 0 0; /* 与 diff 主体留间距 */
  font-size: 12px; /* 小字提示 */
  color: var(--el-color-warning); /* 警告色 */
}
</style>
