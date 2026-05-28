// =====================================================================
// Git 操作相关类型
// 与 src-tauri/src/models/git.rs 对齐。
// =====================================================================

/** 文件变更状态。 */
export type FileStatus =
  | 'untracked'
  | 'added'
  | 'modified'
  | 'staged'
  | 'deleted'
  | 'renamed'
  | 'conflicted'
  | 'ignored';

/** 单个文件变更条目。 */
export interface FileChange {
  path: string;
  oldPath?: string;
  status: FileStatus;
  staged: boolean;
}

/** Git 分支。 */
export interface Branch {
  name: string;
  isCurrent: boolean;
  isRemote: boolean;
  upstream?: string;
  ahead: number;
  behind: number;
  lastCommitShort?: string;
}

/** 提交简要信息。 */
export interface CommitInfo {
  sha: string;
  shortSha: string;
  summary: string;
  message: string;
  authorName: string;
  authorEmail: string;
  authoredAt: string;
  parentShas: string[];
}

/** 工作区聚合状态。 */
export interface GitStatus {
  currentBranch?: string;
  upstream?: string;
  ahead: number;
  behind: number;
  changes: FileChange[];
  isClean: boolean;
}

/** Diff 查询结果（V1 为纯文本，> 1MB 自动截断）。 */
export interface DiffResult {
  text: string;
  truncated: boolean;
}
