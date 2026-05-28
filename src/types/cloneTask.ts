// =====================================================================
// 克隆任务相关类型
// 与 src-tauri/src/models/clone_task.rs 对齐。
// =====================================================================

/** 克隆任务状态。 */
export type CloneTaskStatus = 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';

/** 克隆任务实体。 */
export interface CloneTask {
  id: string;
  remoteRepositoryId: string;
  repositoryName: string;
  remoteUrl: string;
  targetPath: string;
  status: CloneTaskStatus;
  /** 进度百分比 0-100 */
  progress: number;
  errorMessage?: string;
  createdAt: string;
  startedAt?: string;
  finishedAt?: string;
}
