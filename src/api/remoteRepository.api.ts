// =====================================================================
// 远程仓库 API 封装
// 封装与 src-tauri/src/commands/remote_repositories.rs 对应的 5 个 IPC 命令。
// =====================================================================

import { invokeCmd } from './tauri';
import type { CommitDetail, CommitPage } from '@/types/git';
import type { RemoteRepository } from '@/types/repository';

/** 远程仓库筛选条件。 */
export interface RemoteRepoFilter {
  accountId?: string;
  platforms?: string[];
  owners?: string[];
  visibilities?: string[];
  onlyFavorite?: boolean;
  search?: string;
}

export const remoteRepositoryApi = {
  /** 查询远程仓库列表。 */
  list(filter: RemoteRepoFilter = {}): Promise<RemoteRepository[]> {
    return invokeCmd<RemoteRepository[]>('list_remote_repositories', { filter });
  },

  /** 搜索远程仓库。 */
  search(keyword: string, filter: RemoteRepoFilter = {}): Promise<RemoteRepository[]> {
    return invokeCmd<RemoteRepository[]>('search_remote_repositories', { keyword, filter });
  },

  /** 刷新远程仓库（触发同步）。 */
  refresh(accountId?: string): Promise<number> {
    return invokeCmd<number>('refresh_remote_repositories', { accountId: accountId ?? null });
  },

  /** 获取单个远程仓库详情。 */
  getDetail(repoId: string): Promise<RemoteRepository> {
    return invokeCmd<RemoteRepository>('get_remote_repository_detail', { repoId });
  },

  /** 切换收藏状态。 */
  toggleFavorite(repoId: string): Promise<boolean> {
    return invokeCmd<boolean>('toggle_favorite_remote_repository', { repoId });
  },

  /** 拉取远程仓库提交历史（page 从 1 起、缺省每页 30）。 */
  listCommits(repoId: string, page?: number, perPage?: number): Promise<CommitPage> {
    return invokeCmd<CommitPage>('list_remote_commits', {
      repoId,
      page: page ?? null,
      perPage: perPage ?? null,
    });
  },

  /** 获取远程仓库单个提交的详情。 */
  getCommitDetail(repoId: string, sha: string): Promise<CommitDetail> {
    return invokeCmd<CommitDetail>('get_remote_commit_detail', { repoId, sha });
  },
};
