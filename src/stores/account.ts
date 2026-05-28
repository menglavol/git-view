// =====================================================================
// 账号 store（T035 真实化）
// state：accounts / loading / error / defaultAccountId
// actions：loadAccounts / addAccount / removeAccount / setDefault /
//           updateAccount / toggleEnabled
// getters：defaultAccount / accountsByPlatform / gitlabSelfHostedAccounts
// =====================================================================

import { defineStore } from 'pinia';
import { computed, ref } from 'vue';

import { accountApi, type AccountUpdate, type AddAccountPayload } from '@/api/account.api';
import type { Account, GitPlatform } from '@/types/account';

/** GitLab.com 公有云 host（用于区分自建实例）。 */
const GITLAB_COM_HOST = 'gitlab.com';

export const useAccountStore = defineStore('account', () => {
  // ---------------- state ----------------
  const accounts = ref<Account[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);

  // ---------------- getters ----------------
  /** 当前默认账号（无则返回 undefined）。 */
  const defaultAccount = computed<Account | undefined>(() =>
    accounts.value.find((a) => a.isDefault),
  );

  /** 当前默认账号 ID（便于 UI 双向绑定）。 */
  const defaultAccountId = computed<string | null>(() => defaultAccount.value?.id ?? null);

  /** 按平台过滤启用账号。 */
  const accountsByPlatform = (platform: GitPlatform): Account[] =>
    accounts.value.filter((a) => a.platform === platform && a.enabled);

  /** 自建 GitLab 账号（host 非 gitlab.com）。 */
  const gitlabSelfHostedAccounts = computed<Account[]>(() =>
    accounts.value.filter((a) => {
      if (a.platform !== 'gitlab') return false;
      try {
        const url = new URL(a.webBaseUrl);
        return url.hostname !== GITLAB_COM_HOST;
      } catch {
        return false;
      }
    }),
  );

  // ---------------- actions ----------------

  /** 拉取账号列表（首次进入页面 / 增删改后调用）。 */
  async function loadAccounts(): Promise<void> {
    loading.value = true;
    error.value = null;
    try {
      accounts.value = await accountApi.list();
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
      throw e;
    } finally {
      loading.value = false;
    }
  }

  /** 添加账号；成功后把新账号合并入列表（无需重新拉取整张表）。 */
  async function addAccount(payload: AddAccountPayload): Promise<Account> {
    loading.value = true;
    error.value = null;
    try {
      const created = await accountApi.add(payload);
      // 新账号若被设为默认，需把其他账号的 isDefault 重置
      if (created.isDefault) {
        accounts.value = accounts.value.map((a) => ({
          ...a,
          isDefault: false,
        }));
      }
      accounts.value.push(created);
      return created;
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
      throw e;
    } finally {
      loading.value = false;
    }
  }

  /** 删除账号；后端会在必要时转移默认账号，本地按 list 重新拉取保持一致。 */
  async function removeAccount(id: string): Promise<void> {
    loading.value = true;
    error.value = null;
    try {
      await accountApi.delete(id);
      await loadAccounts();
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
      throw e;
    } finally {
      loading.value = false;
    }
  }

  /** 设为默认账号。 */
  async function setDefault(id: string): Promise<void> {
    error.value = null;
    try {
      await accountApi.setDefault(id);
      accounts.value = accounts.value.map((a) => ({
        ...a,
        isDefault: a.id === id,
      }));
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
      throw e;
    }
  }

  /** 更新账号字段；后端可能联动转移默认账号，故重新拉取列表。 */
  async function updateAccount(id: string, fields: AccountUpdate): Promise<Account> {
    error.value = null;
    try {
      const updated = await accountApi.update(id, fields);
      // enabled 切换可能引起默认账号转移，重新拉取整张表更安全
      if (fields.enabled !== undefined) {
        await loadAccounts();
      } else {
        accounts.value = accounts.value.map((a) => (a.id === id ? updated : a));
      }
      return updated;
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
      throw e;
    }
  }

  /** 启用/禁用切换（FR-009）。 */
  async function toggleEnabled(id: string, enabled: boolean): Promise<Account> {
    return updateAccount(id, { enabled });
  }

  return {
    // state
    accounts,
    loading,
    error,
    // getters
    defaultAccount,
    defaultAccountId,
    accountsByPlatform,
    gitlabSelfHostedAccounts,
    // actions
    loadAccounts,
    addAccount,
    removeAccount,
    setDefault,
    updateAccount,
    toggleEnabled,
  };
});
