<!--
  账号管理页（T036）。
  - 顶部操作栏：添加账号按钮
  - el-table 渲染账号列表
  - 操作列：同步 / 设为默认 / 删除；行内启用切换
  - 删除：危险确认对话框（要求输入用户名关键词二次确认）
-->

<template>
  <div class="page-accounts">
    <div class="page-header">
      <h2 class="page-title">账号管理</h2>
      <ElButton type="primary" @click="onAddNew">添加账号</ElButton>
    </div>

    <ElTable
      v-loading="store.loading"
      :data="store.accounts"
      style="width: 100%"
      empty-text="还没有账号，点击右上角添加"
    >
      <ElTableColumn label="头像" width="64">
        <template #default="{ row }">
          <ElAvatar :src="row.avatarUrl" :size="32">
            <span>{{ initialOf(row) }}</span>
          </ElAvatar>
        </template>
      </ElTableColumn>

      <ElTableColumn label="平台" width="96">
        <template #default="{ row }">
          <ElTag :type="platformTag(row.platform)" size="small">
            {{ platformLabel(row.platform) }}
          </ElTag>
        </template>
      </ElTableColumn>

      <ElTableColumn label="用户名" min-width="160">
        <template #default="{ row }">
          <div class="user-cell">
            <span class="user-main">{{ row.username }}</span>
            <span v-if="row.displayName" class="user-sub">
              {{ row.displayName }}
            </span>
          </div>
        </template>
      </ElTableColumn>

      <ElTableColumn label="服务地址" min-width="200">
        <template #default="{ row }">
          <span class="host">{{ hostOf(row) }}</span>
        </template>
      </ElTableColumn>

      <ElTableColumn label="备注" min-width="120">
        <template #default="{ row }">
          {{ row.remark || '—' }}
        </template>
      </ElTableColumn>

      <ElTableColumn label="默认" width="80" align="center">
        <template #default="{ row }">
          <ElTag v-if="row.isDefault" type="success" size="small">默认</ElTag>
          <span v-else>—</span>
        </template>
      </ElTableColumn>

      <ElTableColumn label="启用" width="80" align="center">
        <template #default="{ row }">
          <ElSwitch
            :model-value="row.enabled"
            :loading="row.id === togglingId"
            @change="(val: boolean | string | number) => onToggleEnabled(row, Boolean(val))"
          />
        </template>
      </ElTableColumn>

      <ElTableColumn label="最近同步" width="160">
        <template #default="{ row }">
          {{ formatTime(row.lastSyncAt) }}
        </template>
      </ElTableColumn>

      <ElTableColumn label="操作" width="280" fixed="right">
        <template #default="{ row }">
          <ElButtonGroup>
            <ElButton
              size="small"
              :disabled="!row.enabled"
              :loading="row.id === syncingId"
              @click="onSync(row)"
            >
              同步
            </ElButton>
            <ElButton
              size="small"
              :disabled="!row.enabled || row.isDefault"
              @click="onSetDefault(row)"
            >
              设为默认
            </ElButton>
            <ElButton size="small" type="danger" @click="onDelete(row)"> 删除 </ElButton>
          </ElButtonGroup>
        </template>
      </ElTableColumn>
    </ElTable>

    <AccountFormDialog v-model="dialogVisible" :editing-account="editingAccount" @saved="onSaved" />
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';

import AccountFormDialog from '@/components/account/AccountFormDialog.vue';
import { accountApi } from '@/api/account.api';
import { useAccountStore } from '@/stores/account';
import type { Account, GitPlatform } from '@/types/account';

const store = useAccountStore();
const dialogVisible = ref(false);
const editingAccount = ref<Account | null>(null);
const togglingId = ref<string | null>(null);
const syncingId = ref<string | null>(null);

onMounted(() => {
  void store.loadAccounts().catch((e) => {
    ElMessage.error(`加载账号失败：${e instanceof Error ? e.message : String(e)}`);
  });
});

function initialOf(account: Account): string {
  const src = account.displayName || account.username;
  return src.slice(0, 1).toUpperCase();
}

function hostOf(account: Account): string {
  try {
    return new URL(account.webBaseUrl).hostname;
  } catch {
    return account.webBaseUrl;
  }
}

function platformLabel(p: GitPlatform): string {
  return p === 'github' ? 'GitHub' : p === 'gitlab' ? 'GitLab' : 'Gitee';
}

function platformTag(p: GitPlatform): 'info' | 'warning' | 'danger' {
  return p === 'github' ? 'info' : p === 'gitlab' ? 'warning' : 'danger';
}

function formatTime(iso?: string): string {
  if (!iso) return '从未同步';
  try {
    return new Date(iso).toLocaleString();
  } catch {
    return iso;
  }
}

function onAddNew(): void {
  editingAccount.value = null;
  dialogVisible.value = true;
}

function onSaved(): void {
  // 重新拉取以反映后端可能联动的默认账号变化
  void store.loadAccounts();
}

async function onToggleEnabled(row: Account, enabled: boolean): Promise<void> {
  togglingId.value = row.id;
  try {
    await store.toggleEnabled(row.id, enabled);
    ElMessage.success(enabled ? '账号已启用' : '账号已禁用');
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    ElMessage.error(`切换失败：${msg}`);
  } finally {
    togglingId.value = null;
  }
}

async function onSetDefault(row: Account): Promise<void> {
  try {
    await store.setDefault(row.id);
    ElMessage.success('已设为默认账号');
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    ElMessage.error(`操作失败：${msg}`);
  }
}

async function onSync(row: Account): Promise<void> {
  syncingId.value = row.id;
  try {
    const count = await accountApi.syncRepositories(row.id);
    ElMessage.success(`同步完成（${count} 个仓库）`);
    await store.loadAccounts();
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    ElMessage.error(`同步失败：${msg}`);
  } finally {
    syncingId.value = null;
  }
}

async function onDelete(row: Account): Promise<void> {
  try {
    await ElMessageBox.prompt(
      `此操作将删除账号 ${row.username}（${platformLabel(row.platform)}），同时清除系统凭据；已克隆的本地仓库记录将保留。\n请输入用户名 "${row.username}" 以确认：`,
      '危险操作确认',
      {
        type: 'warning',
        confirmButtonText: '确认删除',
        cancelButtonText: '取消',
        inputPattern: new RegExp(`^${escapeRegex(row.username)}$`),
        inputErrorMessage: '输入的用户名不匹配',
      },
    );
    await store.removeAccount(row.id);
    ElMessage.success('账号已删除');
  } catch (e) {
    if (e === 'cancel' || e === 'close') return;
    const msg = e instanceof Error ? e.message : String(e);
    ElMessage.error(`删除失败：${msg}`);
  }
}

function escapeRegex(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}
</script>

<style scoped>
.page-accounts {
  padding: 8px;
}

.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
}

.page-title {
  margin: 0;
  font-size: 20px;
  font-weight: 600;
}

.user-cell {
  display: flex;
  flex-direction: column;
}

.user-main {
  color: var(--el-text-color-primary);
}

.user-sub {
  font-size: 12px;
  color: var(--el-text-color-secondary);
}

.host {
  font-family: var(--el-font-family-monospace, monospace);
  font-size: 13px;
  color: var(--el-text-color-secondary);
}
</style>
