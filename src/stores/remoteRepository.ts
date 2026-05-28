// =====================================================================
// 远程仓库 store
// state：repos / loading / syncing / syncProgress / filter
// actions：fetchList / refresh / toggleFavorite / setFilter
// =====================================================================

import { defineStore } from 'pinia';
import { ref } from 'vue';

import { remoteRepositoryApi, type RemoteRepoFilter } from '@/api/remoteRepository.api';
import type { RemoteRepository } from '@/types/repository';

export const useRemoteRepositoryStore = defineStore('remoteRepository', () => {
  const repositories = ref<RemoteRepository[]>([]);
  const loading = ref(false);
  const syncing = ref(false);
  const error = ref<string | null>(null);
  const filter = ref<RemoteRepoFilter>({});

  async function fetchList(f?: RemoteRepoFilter): Promise<void> {
    loading.value = true;
    error.value = null;
    try {
      const applied = f ?? filter.value;
      filter.value = applied;
      repositories.value = await remoteRepositoryApi.list(applied);
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
      throw e;
    } finally {
      loading.value = false;
    }
  }

  async function refresh(accountId?: string): Promise<number> {
    syncing.value = true;
    error.value = null;
    try {
      const count = await remoteRepositoryApi.refresh(accountId);
      await fetchList();
      return count;
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
      throw e;
    } finally {
      syncing.value = false;
    }
  }

  async function toggleFavorite(repoId: string): Promise<void> {
    const newState = await remoteRepositoryApi.toggleFavorite(repoId);
    const repo = repositories.value.find((r) => r.id === repoId);
    if (repo) {
      repo.isFavorite = newState;
    }
  }

  function setFilter(f: RemoteRepoFilter): void {
    filter.value = f;
  }

  return {
    repositories,
    loading,
    syncing,
    error,
    filter,
    fetchList,
    refresh,
    toggleFavorite,
    setFilter,
  };
});
