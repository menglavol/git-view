// =====================================================================
// 本地仓库 API 封装
// 对应 src-tauri/src/commands/local_repositories.rs 的 9 个 IPC 命令。
// 命名/参数风格保持与 remoteRepository.api.ts、cloneTask.api.ts 一致。
// =====================================================================

import { invokeCmd } from './tauri';
import type { BatchFetchSummary, LocalRepository } from '@/types/repository';

export const localRepositoryApi = {
  /** 添加单个本地仓库（路径由前端通过 dialog 选目录得到）。 */
  add(path: string): Promise<LocalRepository> {
    return invokeCmd<LocalRepository>('add_local_repository', { path });
  },

  /** 扫描父目录批量添加 Git 仓库。`maxDepth` 缺省时后端使用 5。 */
  scan(root: string, maxDepth?: number): Promise<LocalRepository[]> {
    return invokeCmd<LocalRepository[]>('scan_local_repositories', {
      root,
      maxDepth: maxDepth ?? null,
    });
  },

  /** 查询所有已加入的本地仓库。 */
  list(): Promise<LocalRepository[]> {
    return invokeCmd<LocalRepository[]>('list_local_repositories');
  },

  /** 从列表移除（仅删除数据库记录，不删除磁盘文件）。 */
  remove(id: string): Promise<void> {
    return invokeCmd<void>('remove_local_repository', { id });
  },

  /** 刷新单仓库工作区状态。 */
  refreshOne(id: string): Promise<LocalRepository> {
    return invokeCmd<LocalRepository>('refresh_local_repository_status', { id });
  },

  /** 刷新所有仓库工作区状态。 */
  refreshAll(): Promise<LocalRepository[]> {
    return invokeCmd<LocalRepository[]>('refresh_all_local_repository_status');
  },

  /** 并行 Fetch 多个仓库（后端 Semaphore=4 限并发）。 */
  batchFetch(ids: string[]): Promise<BatchFetchSummary> {
    return invokeCmd<BatchFetchSummary>('batch_fetch_repositories', { ids });
  },

  /** 在系统文件管理器中打开仓库目录。 */
  openFolder(id: string): Promise<void> {
    return invokeCmd<void>('open_repository_folder', { id });
  },

  /** 在系统终端中打开仓库目录。 */
  openTerminal(id: string): Promise<void> {
    return invokeCmd<void>('open_repository_in_terminal', { id });
  },
};
