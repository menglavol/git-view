<!--
  通用危险操作二次确认对话框（T116）。

  使用场景（宪法 Principle III 强制）：
    - 删除账号
    - 丢弃工作区变更 (git discard)
    - 清空旧日志
    - 删除凭据
    - 取消 Clone 任务清理半成品目录

  设计要点：
    - 红色警告标题区，让用户立即识别为破坏性操作
    - 列出"将要影响的对象"清单，让用户清楚后果范围
    - "不可恢复"提示明确告知操作不可撤销
    - 关键词二次确认：用户必须输入指定关键词后才启用"确认"按钮
    - 默认关键词为「删除」中文双字，对中文用户友好且不会误触
-->
<template>
  <!-- el-dialog 受控显示;关闭时不重置状态由父组件 v-model 控制 -->
  <el-dialog
    :model-value="visible"
    :title="title || '危险操作确认'"
    :width="dialogWidth"
    :close-on-click-modal="false"
    :close-on-press-escape="!loading"
    @update:model-value="onUpdateVisible"
  >
    <!-- 红色警告头部条：信号色 + 图标 -->
    <div class="danger-header">
      <span class="danger-icon" aria-hidden="true">⚠</span>
      <strong>此操作不可恢复,请仔细确认</strong>
    </div>

    <!-- 主消息文案 -->
    <p class="danger-message">{{ message }}</p>

    <!-- 可选：影响对象清单（如要丢弃的文件列表、要删除的账号名） -->
    <div v-if="itemsToDelete && itemsToDelete.length > 0" class="items-block">
      <p class="items-title">将影响以下 {{ itemsToDelete.length }} 项:</p>
      <ul class="items-list">
        <li v-for="item in displayedItems" :key="item">{{ item }}</li>
        <li v-if="itemsToDelete.length > maxDisplayedItems" class="more-hint">
          ...还有 {{ itemsToDelete.length - maxDisplayedItems }} 项已省略
        </li>
      </ul>
    </div>

    <!-- 可选：恢复性提示（如"已克隆的本地仓库记录保留"） -->
    <p v-if="recoverabilityHint" class="recovery-hint">
      <span aria-hidden="true">ℹ</span>
      {{ recoverabilityHint }}
    </p>

    <!-- 关键词二次确认输入框 -->
    <div class="confirm-input-block">
      <p class="confirm-label">
        请在下方输入 <strong class="kw">{{ confirmKeyword }}</strong> 以继续:
      </p>
      <el-input
        v-model="typedKeyword"
        :placeholder="confirmKeyword"
        :disabled="loading"
        clearable
      />
    </div>

    <!-- 底部按钮组：取消 + 确认；确认按钮只有在关键词匹配时才启用 -->
    <template #footer>
      <el-button :disabled="loading" @click="onCancel">取消</el-button>
      <el-button type="danger" :disabled="!canConfirm" :loading="loading" @click="onConfirm">
        {{ confirmButtonText || '确认执行' }}
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
/**
 * ConfirmDangerDialog 脚本（T116）。
 * 父组件通过 v-model:visible 控制显示,通过 props 注入文案与影响清单;
 * 二次确认通过后 emit('confirm');取消时 emit('cancel') 并由 v-model 关闭。
 */
import { computed, ref, watch } from 'vue';

/** 父组件传入的 props 集合,使用 withDefaults 处理可选字段。 */
const props = withDefaults(
  defineProps<{
    /** 是否显示对话框（v-model:visible） */
    visible: boolean;
    /** 主消息文案,如"确定要丢弃以下文件的所有变更吗?" */
    message: string;
    /** 对话框标题;缺省"危险操作确认" */
    title?: string;
    /** 将受影响的对象清单（如文件路径列表） */
    itemsToDelete?: string[];
    /** 恢复性提示文案（如"已克隆的本地仓库记录保留"） */
    recoverabilityHint?: string;
    /** 二次确认关键词;默认中文「删除」 */
    confirmKeyword?: string;
    /** 自定义确认按钮文案 */
    confirmButtonText?: string;
    /** 异步操作进行中标志,显示在按钮 loading 并禁用 Esc 关闭 */
    loading?: boolean;
    /** 列表最多直接显示几项,超过则折叠 */
    maxDisplayedItems?: number;
    /** 对话框宽度,根据使用场景可调整 */
    dialogWidth?: string;
  }>(),
  {
    title: '',
    itemsToDelete: () => [],
    recoverabilityHint: '',
    confirmKeyword: '删除',
    confirmButtonText: '',
    loading: false,
    maxDisplayedItems: 8,
    dialogWidth: '480px',
  },
);

/** 对话框事件:更新 visible / 确认 / 取消。 */
const emit = defineEmits<{
  'update:visible': [v: boolean];
  confirm: [];
  cancel: [];
}>();

// 用户输入的关键词,内部 ref 保管;visible 切换时清空避免下次复用残留
const typedKeyword = ref('');

// 监听 visible 切到 true 时清空输入;由父组件 v-model 控制重置时机
watch(
  () => props.visible,
  (v) => {
    if (v) typedKeyword.value = '';
  },
);

/** 仅当用户输入的关键词与配置的 confirmKeyword 严格相等时才允许确认。 */
const canConfirm = computed(() => typedKeyword.value.trim() === props.confirmKeyword);

/** 列表过长时折叠,只显示前 maxDisplayedItems 项,剩余在 li 中给出提示。 */
const displayedItems = computed(() => props.itemsToDelete.slice(0, props.maxDisplayedItems));

/** el-dialog v-model 透传;同时在关闭时清空输入,避免下一次打开残留旧值。 */
function onUpdateVisible(v: boolean): void {
  emit('update:visible', v);
  if (!v) typedKeyword.value = '';
}

/** 取消按钮:emit cancel 并通过 v-model 关闭对话框。 */
function onCancel(): void {
  emit('cancel');
  emit('update:visible', false);
}

/** 确认按钮:必须通过 canConfirm 校验才会触发。 */
function onConfirm(): void {
  if (!canConfirm.value) return;
  emit('confirm');
}
</script>

<style scoped>
/* 红色警告头部条:信号色与主体区分,强调"破坏性" */
.danger-header {
  display: flex; /* 横排图标 + 标题 */
  align-items: center; /* 垂直居中 */
  gap: 8px; /* 图标与文字间距 */
  padding: 8px 12px; /* 内边距 */
  margin-bottom: 12px; /* 与下方消息分隔 */
  background: var(--el-color-danger-light-9); /* 浅红背景 */
  border-left: 3px solid var(--el-color-danger); /* 左侧危险色竖条 */
  border-radius: 4px; /* 圆角柔和 */
  color: var(--el-color-danger); /* 危险红文字 */
}
/* 警告图标稍大,与文字基线对齐 */
.danger-icon {
  font-size: 18px; /* 比正文略大 */
}

/* 主消息文案:略大字号、加上下间距 */
.danger-message {
  font-size: 14px; /* 正文字号 */
  margin: 8px 0; /* 上下间距 */
  line-height: 1.6; /* 行高便于阅读 */
}

/* 受影响对象清单块:与主消息留出间距 */
.items-block {
  margin: 12px 0; /* 与其他段留间距 */
  padding: 8px 12px; /* 块内边距 */
  background: var(--el-bg-color-page); /* 浅背景区分 */
  border-radius: 4px; /* 圆角 */
}
.items-title {
  font-size: 13px; /* 副标题字号 */
  margin: 0 0 4px 0; /* 与列表留间距 */
  color: var(--el-text-color-secondary); /* 次色 */
}
/* 列表使用等宽字体,便于查看文件路径对齐 */
.items-list {
  margin: 0; /* 重置 */
  padding-left: 18px; /* 项目符号缩进 */
  font-family: var(--el-font-family-mono, monospace); /* 等宽 */
  font-size: 12px; /* 紧凑字号 */
  max-height: 160px; /* 限高避免对话框过大 */
  overflow: auto; /* 内部滚动 */
}
/* 折叠提示行:斜体次色,与正常项区分 */
.items-list .more-hint {
  font-style: italic; /* 斜体 */
  color: var(--el-text-color-placeholder); /* 占位色 */
  list-style: none; /* 去掉圆点 */
  margin-left: -18px; /* 抵消 padding-left */
}

/* 恢复性提示:信息蓝调,与红色警告形成视觉对比 */
.recovery-hint {
  display: flex; /* 横排图标与文字 */
  align-items: center; /* 垂直居中 */
  gap: 6px; /* 图标文字间距 */
  font-size: 12px; /* 小字提示 */
  color: var(--el-color-info); /* 信息蓝 */
  margin: 8px 0; /* 上下间距 */
}

/* 关键词输入块:紧贴底部按钮组上方 */
.confirm-input-block {
  margin-top: 12px; /* 与上方留间距 */
}
.confirm-label {
  font-size: 13px; /* 输入说明字号 */
  margin: 0 0 6px 0; /* 与输入框留间距 */
  color: var(--el-text-color-regular); /* 正常字色 */
}
/* 关键词加粗显示,提示用户精确输入 */
.kw {
  color: var(--el-color-danger); /* 危险红突出 */
  font-family: var(--el-font-family-mono, monospace); /* 等宽便于复读 */
}
</style>
