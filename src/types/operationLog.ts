// =====================================================================
// 操作日志类型
// 与 src-tauri/src/models/operation_log.rs 对齐。
// =====================================================================

/**
 * 操作类型（V1 范围，不含 V2 的 merge/rebase）。
 */
export type OperationType =
  | 'add_account'
  | 'delete_account'
  | 'test_connection'
  | 'sync_repos'
  | 'clone'
  | 'fetch'
  | 'pull'
  | 'push'
  | 'commit'
  | 'checkout'
  | 'create_branch'
  | 'scan_repos'
  | 'discard_changes';

/** 操作结果状态。 */
export type OperationStatus = 'success' | 'failed' | 'cancelled';

/** 操作日志条目。 */
export interface OperationLog {
  id: string;
  operationType: OperationType;
  target: string;
  status: OperationStatus;
  errorMessage?: string;
  /** 耗时毫秒 */
  durationMs: number;
  occurredAt: string;
}
