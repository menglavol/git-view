// =====================================================================
// 本地仓库 API 封装
// 对应 src-tauri/src/commands/local_repositories.rs 的 9 个 IPC 命令。
// 命名/参数风格保持与 remoteRepository.api.ts、cloneTask.api.ts 一致。
// =====================================================================

import { invokeCmd } from './tauri';
import type {
  BatchFetchSummary,
  LocalRepository,
  RemoteRepository,
  ScanResult,
  Visibility,
} from '@/types/repository';

/** 「发布到远程」命令参数。 */
export interface PublishLocalRepoPayload {
  /** 本地仓库 id（local_repositories.id） */
  repoId: string;
  /** 目标账户 id：决定发布到哪个平台、用谁的 token 建仓 */
  accountId: string;
  /** 远程仓库名（同时用作 GitLab 的 path） */
  name: string;
  /** 仓库描述，可空 */
  description?: string;
  /** 可见性：public / private / internal */
  visibility: Visibility;
  /** 关联协议：https 注入账户 token，ssh 走本机 key */
  protocol: 'https' | 'ssh';
}

export const localRepositoryApi = {
  /** 添加单个本地仓库（路径由前端通过 dialog 选目录得到）。 */
  add(path: string): Promise<LocalRepository> {
    return invokeCmd<LocalRepository>('add_local_repository', { path });
  },

  /** 扫描父目录：新增 Git 仓库并清理该目录下已失效的记录。`maxDepth` 缺省时后端使用 5。 */
  scan(root: string, maxDepth?: number): Promise<ScanResult> {
    return invokeCmd<ScanResult>('scan_local_repositories', {
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

  /**
   * 把一个尚无 origin 的本地仓库发布到远程平台：建空仓 → 关联 → push。
   * 部分失败（远程已建成功但 push 失败）时后端不回滚，返回带进度说明的错误。
   */
  publish(payload: PublishLocalRepoPayload): Promise<RemoteRepository> {
    return invokeCmd<RemoteRepository>('publish_local_repository', {
      repoId: payload.repoId,
      accountId: payload.accountId,
      name: payload.name,
      description: payload.description ?? null,
      visibility: payload.visibility,
      protocol: payload.protocol,
    });
  },
};
