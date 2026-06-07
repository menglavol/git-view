// =====================================================================
// Git 工作流 API 封装（US5 / T082）
// 对应 src-tauri/src/commands/git.rs 的 15 个 IPC 命令。
// 命名风格保持与 remoteRepository.api.ts、localRepository.api.ts 一致。
// =====================================================================

import { invokeCmd } from './tauri';
import type { Branch, CommitDetail, CommitInfo, DiffResult, GitStatus } from '@/types/git';

export const gitApi = {
  // -------------------- 状态读取 --------------------

  /** 读取工作区状态：当前分支、upstream、ahead/behind、文件变更列表。 */
  status(repoId: string): Promise<GitStatus> {
    return invokeCmd<GitStatus>('git_status', { repoId });
  },

  /**
   * 查看文件 diff。
   * @param file 缺省时返回工作区所有变更的合并 diff
   * @param cached true 时查看暂存区相对 HEAD 的差异；缺省 false
   */
  diff(repoId: string, file?: string, cached?: boolean): Promise<DiffResult> {
    return invokeCmd<DiffResult>('git_diff', {
      repoId,
      file: file ?? null,
      cached: cached ?? null,
    });
  },

  /** 列出所有分支（含远端追踪分支）。 */
  listBranches(repoId: string): Promise<Branch[]> {
    return invokeCmd<Branch[]>('git_list_branches', { repoId });
  },

  /** 分页查询提交历史。`page` 从 0 起；`pageSize` 缺省 50。 */
  log(repoId: string, page?: number, pageSize?: number): Promise<CommitInfo[]> {
    return invokeCmd<CommitInfo[]>('git_log', {
      repoId,
      page: page ?? null,
      pageSize: pageSize ?? null,
    });
  },

  /** 获取单个提交的详情（元信息 + 改动文件 + 每文件 diff）。 */
  commitDetail(repoId: string, sha: string): Promise<CommitDetail> {
    return invokeCmd<CommitDetail>('git_commit_detail', { repoId, sha });
  },

  // -------------------- 暂存区操作 --------------------

  /** 把单个文件加入暂存区。 */
  stageFile(repoId: string, file: string): Promise<void> {
    return invokeCmd<void>('git_stage_file', { repoId, file });
  },

  /** 把单个文件从暂存区移除（保留工作区修改）。 */
  unstageFile(repoId: string, file: string): Promise<void> {
    return invokeCmd<void>('git_unstage_file', { repoId, file });
  },

  /** 把当前工作区所有变更加入暂存区。 */
  stageAll(repoId: string): Promise<void> {
    return invokeCmd<void>('git_stage_all', { repoId });
  },

  /** 清空整个暂存区（保留工作区修改）。 */
  unstageAll(repoId: string): Promise<void> {
    return invokeCmd<void>('git_unstage_all', { repoId });
  },

  // -------------------- 提交 --------------------

  /**
   * 提交已暂存变更。
   * 后端会先执行 5 项前置校验（user.name/email、非 detached HEAD、
   * 非 conflict、已暂存文件 > 0），任一不满足返回 Internal 错误。
   */
  commit(repoId: string, message: string, description?: string): Promise<string> {
    return invokeCmd<string>('git_commit', {
      repoId,
      message,
      description: description ?? null,
    });
  },

  // -------------------- 网络操作 --------------------

  /** `git fetch --all --prune`。 */
  fetch(repoId: string): Promise<string> {
    return invokeCmd<string>('git_fetch', { repoId });
  },

  /** `git pull --ff-only`，遇分叉或冲突返回中文友好错误。 */
  pull(repoId: string): Promise<string> {
    return invokeCmd<string>('git_pull', { repoId });
  },

  /** `git push`，遇拒绝/无 upstream/鉴权失败返回中文友好错误。 */
  push(repoId: string): Promise<string> {
    return invokeCmd<string>('git_push', { repoId });
  },

  // -------------------- 分支管理 --------------------

  /** 切换到指定分支；工作区不干净时后端返回 `DirtyWorkdir` 错误。 */
  checkoutBranch(repoId: string, branch: string): Promise<void> {
    return invokeCmd<void>('git_checkout_branch', { repoId, branch });
  },

  /** 创建新分支，可选立即切换。 */
  createBranch(repoId: string, name: string, checkout?: boolean): Promise<void> {
    return invokeCmd<void>('git_create_branch', {
      repoId,
      name,
      checkout: checkout ?? null,
    });
  },

  // -------------------- 破坏性操作（Principle III）--------------------

  /**
   * 丢弃工作区变更（不可恢复）。
   * 调用方 **必须** 先通过 ConfirmDangerDialog 让用户输入关键词确认，
   * 然后传入 `confirmed: true`。后端在 `confirmed = false` 时立即返回
   * `UserCancelled`，作为双重防御。
   */
  discardChanges(repoId: string, files: string[], confirmed: boolean): Promise<void> {
    return invokeCmd<void>('git_discard_changes', { repoId, files, confirmed });
  },
};
