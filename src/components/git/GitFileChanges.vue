<!--
  文件变更列表组件（T083 — US5）。

  职责：
    - 接收父组件传入的 GitStatus,按"已暂存 / 未暂存"分段展示
    - 每行显示状态字母 (M/A/D/R/U/C) + 文件路径
    - 支持 stage / unstage / discard 三类操作
    - discard 通过 ConfirmDangerDialog 二次确认（Principle III）
    - 提供状态过滤器,只看 modified / untracked / staged 等
    - 通过 emit 通知父组件触发对应 IPC 调用

  与 RepositoryDetail.vue 的协作：
    - 父组件持有 status 数据并在变更后刷新
    - 本组件仅触发事件,不直接调用 gitApi（保持组件单一职责）
-->
<template>
  <div class="git-file-changes">
    <!-- 顶部工具栏：过滤器 + 全局 stage/unstage 操作 -->
    <header class="toolbar">
      <!-- 状态过滤器:让用户聚焦到某一类变更 -->
      <el-select
        v-model="filterStatus"
        size="small"
        placeholder="过滤"
        class="filter-select"
        clearable
      >
        <el-option label="全部" value="all" />
        <el-option label="已修改" value="modified" />
        <el-option label="新增" value="added" />
        <el-option label="删除" value="deleted" />
        <el-option label="重命名" value="renamed" />
        <el-option label="未跟踪" value="untracked" />
        <el-option label="冲突" value="conflicted" />
      </el-select>
      <!-- 全局 stage/unstage:作用于当前过滤后可见的文件 -->
      <el-button size="small" :disabled="unstaged.length === 0" @click="emit('stage-all')">
        全部暂存
      </el-button>
      <el-button size="small" :disabled="staged.length === 0" @click="emit('unstage-all')">
        全部取消暂存
      </el-button>
    </header>

    <!-- ============ 已暂存段 ============ -->
    <section class="section" :class="{ collapsed: !stagedExpanded }">
      <!-- 可点击折叠头:整行可点,带 aria 展开态与三角图标,计数常驻可见 -->
      <h4
        class="section-title"
        role="button"
        tabindex="0"
        :aria-expanded="stagedExpanded"
        @click="stagedExpanded = !stagedExpanded"
        @keydown.enter.prevent="stagedExpanded = !stagedExpanded"
        @keydown.space.prevent="stagedExpanded = !stagedExpanded"
      >
        <span class="collapse-caret" :class="{ expanded: stagedExpanded }">▶</span>
        已暂存 ({{ staged.length }})
      </h4>
      <div v-if="!stagedExpanded" class="collapsed-hint" />
      <div v-else-if="staged.length === 0" class="empty-hint">无</div>
      <ul v-else class="file-list">
        <li
          v-for="file in staged"
          :key="`s-${file.path}`"
          class="file-row"
          @click="emit('view-diff', file.path)"
        >
          <!-- 状态字母:右侧颜色随 status 类型变化 -->
          <span class="status-letter" :class="statusClass(file.status)">{{
            statusLetter(file.status)
          }}</span>
          <!-- 文件路径:可截断,完整路径用 title 工具提示 -->
          <span class="file-path" :title="file.path">{{ file.path }}</span>
          <!-- 行内操作按钮区,阻止冒泡避免触发整行 click(看 diff) -->
          <span class="file-actions" @click.stop>
            <el-button text size="small" @click="emit('unstage-file', file.path)"
              >取消暂存</el-button
            >
          </span>
        </li>
      </ul>
    </section>

    <!-- ============ 未暂存段（含 untracked / modified / deleted / conflicted） ============ -->
    <section class="section" :class="{ collapsed: !unstagedExpanded }">
      <!-- 可点击折叠头:整行可点,带 aria 展开态与三角图标,计数常驻可见 -->
      <h4
        class="section-title"
        role="button"
        tabindex="0"
        :aria-expanded="unstagedExpanded"
        @click="unstagedExpanded = !unstagedExpanded"
        @keydown.enter.prevent="unstagedExpanded = !unstagedExpanded"
        @keydown.space.prevent="unstagedExpanded = !unstagedExpanded"
      >
        <span class="collapse-caret" :class="{ expanded: unstagedExpanded }">▶</span>
        未暂存 ({{ unstaged.length }})
      </h4>
      <div v-if="!unstagedExpanded" class="collapsed-hint" />
      <div v-else-if="unstaged.length === 0" class="empty-hint">无</div>
      <ul v-else class="file-list">
        <li
          v-for="file in unstaged"
          :key="`u-${file.path}`"
          class="file-row"
          @click="emit('view-diff', file.path)"
        >
          <span class="status-letter" :class="statusClass(file.status)">{{
            statusLetter(file.status)
          }}</span>
          <span class="file-path" :title="file.path">{{ file.path }}</span>
          <!-- 行内 stage / discard 按钮;discard 走二次确认 -->
          <span class="file-actions" @click.stop>
            <el-button text size="small" @click="emit('stage-file', file.path)">暂存</el-button>
            <el-button text size="small" type="danger" @click="onRequestDiscard(file.path)"
              >丢弃</el-button
            >
          </span>
        </li>
      </ul>
    </section>

    <!-- 危险操作二次确认:仅当 discardTarget 有值时显示 -->
    <ConfirmDangerDialog
      v-model:visible="discardDialogVisible"
      title="丢弃工作区变更"
      :message="discardMessage"
      :items-to-delete="discardTargets"
      :recoverability-hint="discardRecoveryHint"
      confirm-keyword="丢弃"
      confirm-button-text="确认丢弃"
      @confirm="onConfirmDiscard"
      @cancel="onCancelDiscard"
    />
  </div>
</template>

<script setup lang="ts">
/**
 * GitFileChanges 脚本（T083）。
 * 受控组件:从父组件接收 changes,只通过 emit 通知父组件改动。
 */
import { computed, ref } from 'vue';

import ConfirmDangerDialog from '@/components/common/ConfirmDangerDialog.vue';
import type { FileChange, FileStatus } from '@/types/git';

/** 父组件传入完整 changes 列表（来自 git_status 响应）。 */
const props = defineProps<{ changes: FileChange[] }>();

/** 事件契约:四种文件级动作 + 两种批量动作 + 一个 discard 二次确认结果。 */
const emit = defineEmits<{
  /** 整行单击：在右侧查看该文件的 diff（不改变暂存状态） */
  'view-diff': [path: string];
  'stage-file': [path: string];
  'unstage-file': [path: string];
  'stage-all': [];
  'unstage-all': [];
  /** 用户已在 ConfirmDangerDialog 完成关键词二次确认 */
  'discard-confirmed': [paths: string[]];
}>();

/** 当前过滤条件:'all' 时不过滤,否则按 FileStatus 精确匹配。 */
const filterStatus = ref<'all' | FileStatus>('all');

/**
 * 两个分段（已暂存 / 未暂存）的展开状态,默认均展开。
 *
 * WHY 用组件内 ref 而非持久化:折叠只是「本次会话内的临时视图偏好」,
 * 用户切换文件 / 切换仓库时保持,但重启应用回归默认展开即可,无需落库。
 */
const stagedExpanded = ref(true);
const unstagedExpanded = ref(true);

/** 应用过滤条件后的可见文件列表。 */
const visibleChanges = computed(() => {
  if (filterStatus.value === 'all') return props.changes;
  return props.changes.filter((c) => c.status === filterStatus.value);
});

/** 已暂存子集(staged = true)。 */
const staged = computed(() => visibleChanges.value.filter((c) => c.staged));
/** 未暂存子集(staged = false,含 untracked 与各种工作区变更)。 */
const unstaged = computed(() => visibleChanges.value.filter((c) => !c.staged));

/** FileStatus → 单字母简称,用于行首彩色徽标。 */
function statusLetter(s: FileStatus): string {
  switch (s) {
    case 'added':
      return 'A';
    case 'deleted':
      return 'D';
    case 'renamed':
      return 'R';
    case 'untracked':
      return 'U';
    case 'conflicted':
      return 'C';
    case 'staged':
      return 'S';
    case 'ignored':
      return 'I';
    case 'modified':
    default:
      return 'M';
  }
}

/** 状态字母对应的 CSS class,用于上色区分。 */
function statusClass(s: FileStatus): string {
  return `status-${s}`;
}

// ============================================================
// Discard 流程：通过 ConfirmDangerDialog 完成 Principle III 二次确认
// ============================================================

/** 对话框开关 */
const discardDialogVisible = ref(false);
/** 当前待丢弃的文件路径列表;支持未来扩展为批量丢弃 */
const discardTargets = ref<string[]>([]);
/** 主消息文案 */
const discardMessage = computed(
  () => `即将丢弃以下 ${discardTargets.value.length} 个文件的所有工作区变更,此操作不可恢复。`,
);
/** 恢复性提示:告知用户其他文件不受影响 */
const discardRecoveryHint = '其他文件的变更不会受到影响;已提交的内容仍可通过 git reflog 找回';

/** 触发丢弃流程:打开对话框等待用户输入关键词二次确认。 */
function onRequestDiscard(path: string): void {
  discardTargets.value = [path];
  discardDialogVisible.value = true;
}

/** 用户完成二次确认:emit 给父组件由其调用 gitApi.discardChanges。 */
function onConfirmDiscard(): void {
  emit('discard-confirmed', [...discardTargets.value]);
  discardDialogVisible.value = false;
}

/** 用户取消时清空 targets,避免下次复用残留。 */
function onCancelDiscard(): void {
  discardTargets.value = [];
}
</script>

<style scoped>
/* 主容器:纵向布局,占满父栏剩余空间 */
.git-file-changes {
  display: flex; /* 启用 flex 布局 */
  flex-direction: column; /* 纵向排布 */
  gap: 8px; /* 段落间距 */
  height: 100%; /* 占满父高 */
  min-height: 0; /* 允许内部 flex 子元素在超高时收缩并滚动 */
}

/* 顶部工具栏:横排过滤器与全局按钮 */
.toolbar {
  display: flex; /* 横向排布 */
  gap: 8px; /* 控件间距 */
  align-items: center; /* 垂直居中 */
  flex: 0 0 auto; /* 工具栏定高,不随内容伸缩,把剩余空间让给下方两段 */
}
/* 过滤器固定宽度,避免抖动 */
.filter-select {
  width: 110px; /* 固定宽度让 UI 稳定 */
}

/* 分段:纵向 flex,两段平分剩余高度,各自内部滚动。
   WHY:父列 .col 为 overflow:hidden,若本段不自管溢出,文件多时会把父容器撑破、
   超出部分被裁剪且无滚动条(原缺陷)。故每段 flex:1 + min-height:0,列表再 overflow。 */
.section {
  display: flex; /* 纵向:标题在上,列表在下 */
  flex-direction: column;
  flex: 1; /* 两段平分剩余空间 */
  min-height: 0; /* 关键:允许子列表在超高时收缩触发滚动,而非撑高父容器 */
}
/* 折叠态:不再参与高度平分,收缩到仅标题高,把空间让给展开的另一段 */
.section.collapsed {
  flex: 0 0 auto;
}

/* 分段标题:轻量视觉分割,作为可点击折叠头 */
.section-title {
  font-size: 13px; /* 略小于正文,作为副标题 */
  color: var(--el-text-color-secondary); /* 次色 */
  margin: 6px 0 4px 0; /* 上下间距收紧 */
  flex: 0 0 auto; /* 标题定高,不被列表挤压 */
  display: flex; /* 三角图标与文字横向对齐 */
  align-items: center;
  gap: 4px; /* 图标与文字间距 */
  cursor: pointer; /* 提示可点击折叠 */
  user-select: none; /* 折叠交互不选中文字 */
}
.section-title:hover {
  color: var(--el-text-color-primary); /* hover 提示可交互 */
}
/* 折叠三角:展开态旋转 90° 指向下方,带过渡 */
.collapse-caret {
  display: inline-block;
  font-size: 10px; /* 小三角,不抢视觉 */
  transition: transform 0.15s; /* 展开/折叠平滑旋转 */
  color: var(--el-text-color-placeholder);
}
.collapse-caret.expanded {
  transform: rotate(90deg); /* 展开时三角朝下 */
}
/* 折叠态占位:保持段间 gap 视觉稳定(无内容但不塌陷) */
.collapsed-hint {
  height: 0;
}

/* 空状态文案:次色斜体 */
.empty-hint {
  color: var(--el-text-color-placeholder); /* 占位色 */
  font-size: 12px; /* 小字 */
  font-style: italic; /* 斜体区分提示态 */
  padding-left: 8px; /* 与列表项对齐 */
}

/* 文件列表:取消默认 ul 样式,并在段内独立滚动。
   flex:1 + min-height:0 让列表吃满段内剩余高度;overflow-y:auto 使文件多时
   在段内滚动而非把 .section 撑高——这是「变更列表完整可见」的关键修复。 */
.file-list {
  list-style: none; /* 去掉项目符号 */
  padding: 0; /* 重置 */
  margin: 0; /* 重置 */
  flex: 1; /* 吃满段内剩余高度 */
  min-height: 0; /* 允许收缩以触发自身滚动 */
  overflow-y: auto; /* 文件超高时段内滚动 */
}

/* 单行:横排 + 悬停高亮提示可点击 */
.file-row {
  display: flex; /* 横排 status / path / actions */
  align-items: center; /* 垂直居中 */
  gap: 8px; /* 列间距 */
  padding: 4px 6px; /* 紧凑行高 */
  border-radius: 3px; /* 圆角让 hover 视觉柔和 */
  cursor: pointer; /* 提示可点击 */
  font-size: 12px; /* 列表小字 */
}
.file-row:hover {
  background: var(--el-fill-color-light); /* hover 浅背景 */
}

/* 状态字母徽标:宽度固定形成网格感 */
.status-letter {
  display: inline-block; /* 让 width 生效 */
  width: 16px; /* 固定宽度对齐 */
  text-align: center; /* 居中显示字母 */
  font-family: var(--el-font-family-mono, monospace); /* 等宽字体 */
  font-weight: bold; /* 加粗突出 */
}
/* 各状态颜色对应:绿/红/黄/灰 体系 */
.status-added,
.status-untracked {
  color: var(--el-color-success); /* 新增/未跟踪用成功绿 */
}
.status-deleted {
  color: var(--el-color-danger); /* 删除用危险红 */
}
.status-modified,
.status-renamed,
.status-staged {
  color: var(--el-color-warning); /* 修改/重命名/已暂存用警告黄 */
}
.status-conflicted {
  color: var(--el-color-error); /* 冲突用错误色 */
}
.status-ignored {
  color: var(--el-text-color-placeholder); /* 忽略用次色 */
}

/* 文件路径:占据中间剩余空间,长路径截断 */
.file-path {
  flex: 1; /* 撑满剩余空间 */
  font-family: var(--el-font-family-mono, monospace); /* 等宽便于读路径 */
  white-space: nowrap; /* 不换行 */
  overflow: hidden; /* 隐藏超出 */
  text-overflow: ellipsis; /* 末尾省略号 */
}

/* 行内操作区:hover 才显示,减少视觉干扰 */
.file-actions {
  opacity: 0; /* 默认隐藏 */
  transition: opacity 0.15s; /* 渐显平滑 */
}
.file-row:hover .file-actions {
  opacity: 1; /* hover 时显示按钮 */
}
</style>
