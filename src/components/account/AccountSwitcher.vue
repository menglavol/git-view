<!--
  顶栏账号切换器（下拉）。
  展示当前默认账号；点击展开后可切换默认账号或跳转到账号管理页。
-->

<template>
  <ElDropdown
    v-if="store.accounts.length > 0"
    trigger="click"
    placement="bottom-end"
    @command="onSelect"
  >
    <span class="switcher-trigger">
      <ElAvatar :src="current?.avatarUrl" :size="28">
        <span>{{ currentInitial }}</span>
      </ElAvatar>
      <span class="current-name">
        {{ current?.displayName || current?.username || '未选择默认账号' }}
      </span>
      <span class="caret">▾</span>
    </span>
    <template #dropdown>
      <ElDropdownMenu>
        <ElDropdownItem
          v-for="acc in enabledAccounts"
          :key="acc.id"
          :command="acc.id"
          :disabled="acc.isDefault"
        >
          <AccountCard :account="acc" />
        </ElDropdownItem>
        <ElDropdownItem divided command="__manage__">
          <span>管理账号</span>
        </ElDropdownItem>
      </ElDropdownMenu>
    </template>
  </ElDropdown>

  <ElButton v-else type="primary" size="small" @click="$router.push({ name: 'accounts' })">
    添加账号
  </ElButton>
</template>

<script setup lang="ts">
import { computed, onMounted } from 'vue';
import { useRouter } from 'vue-router';
import { ElMessage } from 'element-plus';

import AccountCard from './AccountCard.vue';
import { useAccountStore } from '@/stores/account';

const store = useAccountStore();
const router = useRouter();

const current = computed(() => store.defaultAccount);

const currentInitial = computed(() => {
  const src = current.value?.displayName || current.value?.username || '?';
  return src.slice(0, 1).toUpperCase();
});

const enabledAccounts = computed(() => store.accounts.filter((a) => a.enabled));

onMounted(() => {
  if (store.accounts.length === 0) {
    void store.loadAccounts().catch(() => {
      // 启动时静默失败：账号列表加载失败不影响其他模块
    });
  }
});

async function onSelect(command: string | number | object): Promise<void> {
  const cmd = String(command);
  if (cmd === '__manage__') {
    await router.push({ name: 'accounts' });
    return;
  }
  try {
    await store.setDefault(cmd);
    ElMessage.success('已切换默认账号');
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    ElMessage.error(`切换失败：${msg}`);
  }
}
</script>

<style scoped>
.switcher-trigger {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 4px;
  user-select: none;
}

.switcher-trigger:hover {
  background-color: var(--el-fill-color-light);
}

.current-name {
  font-size: 14px;
  color: var(--el-text-color-primary);
  max-width: 160px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.caret {
  color: var(--el-text-color-secondary);
  font-size: 12px;
}
</style>
