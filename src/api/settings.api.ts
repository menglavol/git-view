// =====================================================================
// 设置 API 封装
// 封装与 src-tauri/src/commands/settings.rs 对应的 4 个 IPC 命令。
// 调用方只处理强类型 Promise 与 GitViewClientError，无需关心 invoke 细节。
// =====================================================================

import { invokeCmd } from './tauri';
import type { GitDetectionResult, Settings } from '@/types/settings';

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
};
