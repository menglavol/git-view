<!--
  账号卡片（紧凑展示）。
  用于：账号列表、AccountSwitcher 下拉等位置复用。
  字段：头像、用户名、平台标签、默认标识、备注。
-->

<template>
  <div class="account-card" :class="{ default: account.isDefault, disabled: !account.enabled }">
    <ElAvatar :src="account.avatarUrl" :size="32" class="avatar">
      <span>{{ initials }}</span>
    </ElAvatar>
    <div class="meta">
      <div class="name-line">
        <span class="username">{{ account.displayName || account.username }}</span>
        <ElTag v-if="account.isDefault" type="success" size="small">默认</ElTag>
        <ElTag v-if="!account.enabled" type="info" size="small">已禁用</ElTag>
      </div>
      <div class="sub-line">
        <ElTag :type="platformTagType" size="small" effect="plain">
          {{ platformLabel }}
        </ElTag>
        <span class="host">{{ host }}</span>
      </div>
      <div v-if="account.remark" class="remark">{{ account.remark }}</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import type { Account, GitPlatform } from '@/types/account';

const props = defineProps<{ account: Account }>();

const initials = computed(() => {
  const src = props.account.displayName || props.account.username;
  return src.slice(0, 1).toUpperCase();
});

const host = computed(() => {
  try {
    return new URL(props.account.webBaseUrl).hostname;
  } catch {
    return props.account.webBaseUrl;
  }
});

const platformLabel = computed(() => labelFor(props.account.platform));

const platformTagType = computed<'success' | 'warning' | 'danger' | 'info'>(() => {
  switch (props.account.platform) {
    case 'github':
      return 'info';
    case 'gitlab':
      return 'warning';
    case 'gitee':
      return 'danger';
    default:
      return 'info';
  }
});

function labelFor(p: GitPlatform): string {
  switch (p) {
    case 'github':
      return 'GitHub';
    case 'gitlab':
      return 'GitLab';
    case 'gitee':
      return 'Gitee';
  }
}
</script>

<style scoped>
.account-card {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 8px;
  border-radius: 6px;
}

.account-card.default {
  background-color: var(--el-color-success-light-9);
}

.account-card.disabled {
  opacity: 0.6;
}

.avatar {
  flex-shrink: 0;
}

.meta {
  flex: 1;
  min-width: 0;
}

.name-line {
  display: flex;
  align-items: center;
  gap: 8px;
}

.username {
  font-weight: 600;
  color: var(--el-text-color-primary);
}

.sub-line {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 4px;
  font-size: 12px;
  color: var(--el-text-color-secondary);
}

.host {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.remark {
  margin-top: 4px;
  font-size: 12px;
  color: var(--el-text-color-secondary);
}
</style>
