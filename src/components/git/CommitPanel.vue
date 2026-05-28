<!--
  Commit 提交面板组件（T085 — US5）。

  职责：
    - 提供 message 单行 + description 多行输入
    - 字符计数 + 提交按钮禁用规则
    - 父组件传入 status / userConfigOk 让本组件判断按钮可用性
    - 提交成功后 emit('committed') 由父组件刷新工作区状态

  禁用条件（FR-038）：
    - 无已暂存文件
    - message 为空
    - 后端校验失败时由父组件捕获 Internal 错误并 ElMessage 提示
-->
<template>
  <div class="commit-panel">
    <!-- 单行 message 输入:Conventional Commits 风格鼓励简短 -->
    <el-input
      v-model="message"
      placeholder="commit 标题(必填,建议 50 字以内)"
      :disabled="submitting"
      :maxlength="500"
      show-word-limit
      size="small"
    />

    <!-- 多行 description 输入:可选,放在标题下方 -->
    <el-input
      v-model="description"
      type="textarea"
      :rows="6"
      placeholder="详细描述(可选,留空则只提交标题)"
      :disabled="submitting"
      :maxlength="5000"
      show-word-limit
      class="description-input"
    />

    <!-- 校验提示:列出当前阻断条件,帮助用户判断为何按钮不可用 -->
    <ul v-if="blockingReasons.length > 0" class="block-reasons">
      <li v-for="reason in blockingReasons" :key="reason">{{ reason }}</li>
    </ul>

    <!-- 底部按钮 -->
    <div class="actions">
      <el-button type="primary" :disabled="!canCommit" :loading="submitting" @click="onSubmit">
        提交
      </el-button>
    </div>
  </div>
</template>

<script setup lang="ts">
/**
 * CommitPanel 脚本（T085）。
 * 受控组件:从父组件接收 stagedCount / submitting,通过 emit 上抛提交意图。
 */
import { computed, ref } from 'vue';

const props = defineProps<{
  /** 已暂存文件数,= 0 时阻断提交 */
  stagedCount: number;
  /** 父组件标记的提交进行中状态,用于按钮 loading */
  submitting: boolean;
}>();

const emit = defineEmits<{
  /** 用户触发提交:父组件实际调用 gitApi.commit 并捕获错误 */
  submit: [payload: { message: string; description?: string }];
}>();

/** message 与 description 内部 ref,提交成功后由父组件通过 resetInputs 重置 */
const message = ref('');
const description = ref('');

/** 提交按钮可用条件:无 submitting && 有已暂存文件 && message 非空 */
const canCommit = computed(
  () => !props.submitting && props.stagedCount > 0 && message.value.trim().length > 0,
);

/** 阻断原因列表:动态生成给用户友好提示。 */
const blockingReasons = computed(() => {
  const reasons: string[] = [];
  if (props.stagedCount === 0) reasons.push('没有已暂存的文件,请先 stage 要提交的变更');
  if (message.value.trim().length === 0) reasons.push('请填写 commit 标题');
  return reasons;
});

/** 触发提交事件;成功后父组件调用 reset() 清空输入。 */
function onSubmit(): void {
  if (!canCommit.value) return;
  emit('submit', {
    message: message.value.trim(),
    description: description.value.trim() || undefined,
  });
}

/** 暴露给父组件:提交成功后调用 reset 清空输入。 */
defineExpose({
  reset(): void {
    message.value = '';
    description.value = '';
  },
});
</script>

<style scoped>
/* 主容器:纵向布局,各输入框等间距 */
.commit-panel {
  display: flex; /* 启用 flex 用于纵排 */
  flex-direction: column; /* 上下堆叠输入与按钮 */
  gap: 8px; /* 控件间距 */
  height: 100%; /* 撑满父高 */
}

/* 描述输入框:让 textarea 占用更多高度 */
.description-input {
  flex: 1; /* 占据剩余空间 */
}
/* el-textarea 内部需要让 textarea 元素也跟随高度 */
.description-input :deep(.el-textarea__inner) {
  height: 100%; /* 内部 textarea 跟随父高 */
}

/* 阻断原因清单:橙色提示,无项目符号 */
.block-reasons {
  list-style: none; /* 去掉默认圆点 */
  padding: 0; /* 重置 */
  margin: 0; /* 重置 */
  font-size: 12px; /* 小字提示 */
  color: var(--el-color-warning); /* 警告黄 */
}
.block-reasons li::before {
  content: '⚠ '; /* 警告前缀字符 */
}

/* 底部按钮区:右对齐 */
.actions {
  display: flex; /* flex 让按钮对齐 */
  justify-content: flex-end; /* 按钮靠右 */
}
</style>
