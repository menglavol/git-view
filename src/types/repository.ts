// =====================================================================
// 仓库相关类型定义
// 与 src-tauri/src/models/repository.rs 对齐。
// =====================================================================

import type { GitPlatform } from './account';

/** 仓库可见性。 */
export type Visibility = 'public' | 'private' | 'internal';

/** 本地仓库工作区状态摘要。 */
export type RepositoryStatus = 'clean' | 'dirty' | 'ahead' | 'behind' | 'diverged' | 'unknown';

/** 远程仓库元数据。 */
export interface RemoteRepository {
  id: string;
  accountId: string;
  platform: GitPlatform;
  remoteId: string;
  fullName: string;
  name: string;
  owner: string;
  description?: string;
  visibility: Visibility;
  defaultBranch: string;
  htmlUrl: string;
  sshUrl?: string;
  cloneUrl: string;
  isFavorite: boolean;
  lastPushedAt?: string;
  syncedAt: string;
}

/** 本地仓库元数据。 */
export interface LocalRepository {
  id: string;
  remoteRepositoryId?: string;
  localPath: string;
  currentBranch?: string;
  remoteUrl?: string;
  status: RepositoryStatus;
  lastCheckedAt: string;
  createdAt: string;
}

/** 扫描父目录的结果：新增仓库列表 + 清理掉的失效记录数。 */
export interface ScanResult {
  /** 本次新增的本地仓库（不含已存在的） */
  added: LocalRepository[];
  /** 本次清理掉的失效记录数（在扫描父目录之下、磁盘已不存在） */
  removed: number;
}

/** 单个仓库 Fetch 失败明细。 */
export interface FetchFailure {
  /** 仓库 id（local_repositories.id） */
  repoId: string;
  /** 仓库名（V1 后端先填本地路径，前端展示时取末段目录名） */
  repoName: string;
  /** 已脱敏的错误描述 */
  error: string;
}

/** 批量 Fetch 汇总结果。 */
export interface BatchFetchSummary {
  total: number;
  success: number;
  failed: number;
  failures: FetchFailure[];
}
