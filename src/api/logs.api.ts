// =====================================================================
// 操作日志 API 封装（T097 / US6）
// 对应 src-tauri/src/commands/logs.rs 的 3 个 IPC 命令。
// 命名/参数风格保持与 localRepository.api.ts、cloneTask.api.ts 一致。
// =====================================================================

import { invokeCmd } from './tauri';
import type { LogFilter, OperationLog } from '@/types/operationLog';

export const logsApi = {
  /** 按筛选条件分页查询操作日志（后端按发生时间倒序，含中文错误翻译）。 */
  list(filter: LogFilter): Promise<OperationLog[]> {
    return invokeCmd<OperationLog[]>('list_operation_logs', { filter });
  },

  /** 查询单条日志详情；不存在时后端返回 null。 */
  getDetail(id: string): Promise<OperationLog | null> {
    return invokeCmd<OperationLog | null>('get_operation_log_detail', { id });
  },

  /**
   * 清理操作日志（破坏性，前端调用前须经 ConfirmDangerDialog 二次确认）。
   *
   * @param beforeDays 缺省 / undefined 清空全部；传 n 删除 n 天前的日志。
   * @returns 删除的行数。
   */
  clearOld(beforeDays?: number): Promise<number> {
    return invokeCmd<number>('clear_old_operation_logs', {
      beforeDays: beforeDays ?? null,
    });
  },
};
