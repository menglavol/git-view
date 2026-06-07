// =====================================================================
// 设置 API 封装
// 封装与 src-tauri/src/commands/settings.rs 对应的 4 个 IPC 命令。
// 调用方只处理强类型 Promise 与 GitViewClientError，无需关心 invoke 细节。
// =====================================================================

import { invokeCmd } from './tauri';
import type {
  ClearLogsResult,
  DataDirInfo,
  GitDetectionResult,
  LogStats,
  MigrateResult,
  OldDataDir,
  Settings,
} from '@/types/settings';

/**
 * 设置相关 API 集合。
 *
 * get/update 操作完整设置快照；detect/setGitPath 处理 Git 环境探测。
 */
export const settingsApi = {
  /** 读取完整设置快照（缺失项后端回退默认值）。 */
  getSettings(): Promise<Settings> {
    return invokeCmd<Settings>('get_settings');
  },

  /** 原子写入完整设置快照。 */
  updateSettings(settings: Settings): Promise<void> {
    return invokeCmd<void>('update_settings', { settings });
  },

  /** 探测可用的 git（优先用设置中保存的路径，失败回退自动探测）。 */
  detectGit(): Promise<GitDetectionResult> {
    return invokeCmd<GitDetectionResult>('detect_git');
  },

  /** 校验用户指定的 git 路径并持久化（校验失败时抛错）。 */
  setGitPath(path: string): Promise<GitDetectionResult> {
    return invokeCmd<GitDetectionResult>('set_git_path', { path });
  },

  /** 读取日志目录占用统计（路径 + 大小 + 文件数）。 */
  getLogStats(): Promise<LogStats> {
    return invokeCmd<LogStats>('get_log_stats');
  },

  /** 清理历史日志（保留当天、删除更早的滚动文件），返回删除数与释放字节。 */
  clearOldLogs(): Promise<ClearLogsResult> {
    return invokeCmd<ClearLogsResult>('clear_old_logs');
  },

  /** 读取当前数据目录信息（当前路径 + 是否存在待删旧目录）。 */
  getDataDir(): Promise<DataDirInfo> {
    return invokeCmd<DataDirInfo>('get_data_dir');
  },

  /** 迁移数据目录：把当前 DB + 日志复制到 newDir，更新指针（需重启生效）。 */
  migrateDataDir(newDir: string): Promise<MigrateResult> {
    return invokeCmd<MigrateResult>('migrate_data_dir', { newDir });
  },

  /** 恢复到应用默认数据目录（把当前数据复制回默认目录，需重启生效）。 */
  restoreDefaultDataDir(): Promise<MigrateResult> {
    return invokeCmd<MigrateResult>('restore_default_data_dir');
  },

  /** 读取旧数据目录占用（路径 + 大小 + 文件数）；无旧目录时返回 null。 */
  getOldDataDir(): Promise<OldDataDir | null> {
    return invokeCmd<OldDataDir | null>('get_old_data_dir');
  },

  /** 删除旧数据目录并清空指针 previousDir。 */
  deleteOldDataDir(): Promise<void> {
    return invokeCmd<void>('delete_old_data_dir');
  },

  /** 重启应用（迁移后使新数据目录生效）。 */
  restartApp(): Promise<void> {
    return invokeCmd<void>('restart_app');
  },
};
