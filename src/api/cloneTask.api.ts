// =====================================================================
// Clone 任务 API 封装
// 对应 src-tauri/src/commands/clone_tasks.rs 的 6 个 IPC 命令。
// =====================================================================

import { invokeCmd, listenEvent } from './tauri';
import type { CloneTask, CloneTaskStatus } from '@/types/cloneTask';
import type { DirectoryStrategy } from '@/types/settings';

/** 创建任务的 payload。 */
export interface CreateCloneTasksPayload {
  remoteRepositoryIds: string[];
  targetRoot: string;
  directoryStrategy: DirectoryStrategy;
  concurrency?: number;
  autoAddToLocal: boolean;
}

/** Clone 进度事件。 */
export interface CloneProgressPayload {
  taskId: string;
  stage: string;
  percent: number;
}

/** Clone 状态变更事件。 */
export interface CloneStatusPayload {
  taskId: string;
  status: CloneTaskStatus;
  progress: number;
  errorMessage?: string;
}

export const cloneTaskApi = {
  create(payload: CreateCloneTasksPayload): Promise<CloneTask[]> {
    return invokeCmd<CloneTask[]>('create_clone_tasks', { payload });
  },
  start(taskIds: string[], autoAddToLocal: boolean): Promise<void> {
    return invokeCmd<void>('start_clone_tasks', { taskIds, autoAddToLocal });
  },
  list(): Promise<CloneTask[]> {
    return invokeCmd<CloneTask[]>('list_clone_tasks');
  },
  cancel(taskId: string): Promise<void> {
    return invokeCmd<void>('cancel_clone_task', { taskId });
  },
  retry(taskId: string, autoAddToLocal: boolean): Promise<void> {
    return invokeCmd<void>('retry_clone_task', { taskId, autoAddToLocal });
  },
  clearByStatus(status: CloneTaskStatus): Promise<number> {
    return invokeCmd<number>('clear_clone_tasks_by_status', { status });
  },

  onProgress(handler: (p: CloneProgressPayload) => void) {
    return listenEvent<CloneProgressPayload>('clone-task-progress', handler);
  },
  onStatusChanged(handler: (p: CloneStatusPayload) => void) {
    return listenEvent<CloneStatusPayload>('clone-task-status-changed', handler);
  },
};
