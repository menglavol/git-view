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

/** 提交列表项（远程提交历史；本地由 CommitInfo 映射成同形态）。 */
export interface CommitSummary {
  sha: string;
  shortSha: string;
  summary: string;
  authorName: string;
  authoredAt: string;
  htmlUrl?: string;
}

/** 提交分页结果（远程 list_remote_commits 返回）。 */
export interface CommitPage {
  items: CommitSummary[];
  hasNext: boolean;
}

/** 单个提交改动文件的状态。 */
export type CommitFileStatus = 'added' | 'modified' | 'deleted' | 'renamed';

/** 提交改动文件（含每文件 diff；GitLab 的增删数可能缺省）。 */
export interface CommitFile {
  path: string;
  oldPath?: string;
  status: CommitFileStatus;
  additions?: number;
  deletions?: number;
  diff?: string;
  truncated: boolean;
}

/** 提交增删行汇总。 */
export interface CommitStats {
  additions: number;
  deletions: number;
  total: number;
}

/** 提交详情（远程与本地共用：元信息 + 改动文件 + 每文件 diff）。 */
export interface CommitDetail {
  sha: string;
  shortSha: string;
  message: string;
  authorName: string;
  authorEmail: string;
  authoredAt: string;
  committerName?: string;
  committerEmail?: string;
  committedAt?: string;
  parentShas: string[];
  htmlUrl?: string;
  stats?: CommitStats;
  files: CommitFile[];
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
