<!--
  分支选择器组件（T086 — US5）。

  职责（spec FR-043 / FR-044）：
    - 显示当前分支名
    - 下拉列表分本地 / 远程两段
    - 点击本地分支调用 checkout_branch
    - 点击远程分支调用 create_branch(checkout=true) 自动 upstream
    - 脏工作区时禁用切换按钮 + tooltip 提示

  与父组件协作：
    - props.branches: 父组件通过 gitApi.listBranches 加载
    - props.isDirty: 父组件计算 status.isClean === false
    - emit('switch-local', name) / emit('switch-remote', remoteName)
    - 切换成功由父组件调用 reload() 刷新本组件数据
-->
<template>
  <el-tooltip :content="dirtyTooltip" :disabled="!isDirty" placement="bottom">
    <el-dropdown trigger="click" :disabled="isDirty" @command="onCommand">
      <!-- 当前分支按钮:作为下拉触发器 -->
      <el-button size="small" :disabled="isDirty">
        分支:{{ currentName || '未知' }}
        <span class="icon" aria-hidden="true">▾</span>
      </el-button>
      <template #dropdown>
        <el-dropdown-menu>
          <!-- 本地分支段标题 -->
          <el-dropdown-item disabled class="section-title">本地分支</el-dropdown-item>
          <el-dropdown-item
            v-for="b in localBranches"
            :key="`l-${b.name}`"
            :command="{ kind: 'local', name: b.name }"
            :class="{ active: b.isCurrent }"
          >
            <span v-if="b.isCurrent" class="check" aria-hidden="true">✓</span>
            <span class="branch-name">{{ b.name }}</span>
          </el-dropdown-item>

          <!-- 远程分支段标题 -->
          <el-dropdown-item disabled class="section-title">远程分支</el-dropdown-item>
          <el-dropdown-item
            v-for="b in remoteBranches"
            :key="`r-${b.name}`"
            :command="{ kind: 'remote', name: b.name }"
          >
            <span class="branch-name">{{ b.name }}</span>
            <span class="hint">从此分支创建本地</span>
          </el-dropdown-item>
        </el-dropdown-menu>
      </template>
    </el-dropdown>
  </el-tooltip>
</template>

<script setup lang="ts">
/**
 * BranchSelector 脚本（T086）。
 * 严格遵循 FR-044:脏工作区时按钮 disabled + tooltip 文案明确。
 */
import { computed } from 'vue';

import type { Branch } from '@/types/git';

const props = defineProps<{
  /** 父组件加载的分支列表(本地 + 远程) */
  branches: Branch[];
  /** 当前 HEAD 分支名;来源 git_status.current_branch */
  currentName: string;
  /** 工作区是否脏(有未提交变更),= !status.is_clean */
  isDirty: boolean;
}>();

const emit = defineEmits<{
  /** 切换本地分支:父组件调用 gitApi.checkoutBranch */
  'switch-local': [name: string];
  /** 从远程分支 checkout 出新本地分支(自动 upstream) */
  'switch-remote': [remoteName: string];
}>();

/** 本地分支子集:过滤 isRemote 字段 */
const localBranches = computed(() => props.branches.filter((b) => !b.isRemote));
/** 远程分支子集:过滤 isRemote = true */
const remoteBranches = computed(() => props.branches.filter((b) => b.isRemote));

/** 脏工作区 tooltip 文案:对应 FR-044 与 spec Acceptance Scenario 11 */
const dirtyTooltip = '存在未提交变更,请先提交或暂存 (stash) 后再切换分支';

/** dropdown command 处理:根据 kind 触发对应 emit。 */
function onCommand(cmd: { kind: 'local' | 'remote'; name: string }): void {
  if (cmd.kind === 'local') {
    // 同名分支无需切换,避免无意义调用 git
    if (cmd.name === props.currentName) return;
    emit('switch-local', cmd.name);
  } else {
    emit('switch-remote', cmd.name);
  }
}
</script>

<style scoped>
/* 下拉触发按钮的箭头图标 */
.icon {
  margin-left: 4px; /* 与文字保持间距 */
}

/* 段标题:禁用项 + 加粗,作为视觉分组 */
.section-title {
  font-weight: 600; /* 加粗突出 */
  font-size: 12px; /* 小字 */
  color: var(--el-text-color-secondary); /* 次色 */
  pointer-events: none; /* 不响应点击,作为分组标题 */
}

/* 当前分支高亮 */
.active {
  color: var(--el-color-primary); /* 主色高亮当前分支 */
}

/* 当前分支左侧勾选图标 */
.check {
  margin-right: 4px; /* 与分支名间距 */
}

/* 分支名:等宽字体便于读取 */
.branch-name {
  font-family: var(--el-font-family-mono, monospace); /* 等宽字体 */
}

/* 远程分支后的提示:小字次色 */
.hint {
  margin-left: 8px; /* 与分支名留间距 */
  font-size: 11px; /* 小字 */
  color: var(--el-text-color-placeholder); /* 占位色 */
}
</style>
