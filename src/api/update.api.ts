// =====================================================================
// 应用内更新 API 封装
// 收拢 @tauri-apps/plugin-updater 与 @tauri-apps/plugin-process 的调用，
// 让组件只依赖本层、不直接 import 插件——与既有 settings.api.ts /
// account.api.ts 的「api 层收拢底层」风格保持一致，便于后续替换实现与统一测试。
//
// 契约来源：specs/003-app-auto-update/contracts/updater-flow.md
// =====================================================================

import { check, type Update, type DownloadEvent } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

/**
 * 下载进度回调：已下载字节 / 总字节。
 *
 * 总字节可能为 null（服务端未返回 Content-Length），此时 UI 需按
 * 「不确定进度」态处理，不强行换算百分比。
 */
export type UpdateProgress = (downloaded: number, total: number | null) => void;

/**
 * 更新相关 API 集合。
 *
 * - check：检查是否有新版本（版本比较由插件内部完成，客户端不自写语义化比较）
 * - downloadAndInstall：下载 + minisign 验签 + 安装，并归一进度事件
 * - relaunch：安装完成后重启应用使新版本生效
 *
 * 本层不吞异常，统一向上抛给组件按「失败不阻断」提示（对齐既有 api 层风格）。
 */
export const updateApi = {
  /** 检查是否有新版本；有则返回 Update 句柄，已是最新返回 null。 */
  check(): Promise<Update | null> {
    return check();
  },

  /**
   * 下载并安装给定更新，通过 onProgress 回报进度。
   *
   * 内部把插件的 Started/Progress/Finished 三态事件归一为「已下载 / 总量」两个数：
   *   - Started：拿到待下载总字节（contentLength）
   *   - Progress：多次触发，累加每次增量（chunkLength）得已下载量
   *   - Finished：下载完成，进度置满后进入安装（插件自动验签）
   */
  async downloadAndInstall(update: Update, onProgress: UpdateProgress): Promise<void> {
    let downloaded = 0; // 已下载累计字节
    let total: number | null = null; // 待下载总字节（未知时为 null）
    await update.downloadAndInstall((event: DownloadEvent) => {
      if (event.event === 'Started') {
        total = event.data.contentLength ?? null; // 起始拿到总大小
      } else if (event.event === 'Progress') {
        downloaded += event.data.chunkLength; // 累加本次增量
        onProgress(downloaded, total);
      } else if (event.event === 'Finished') {
        onProgress(total ?? downloaded, total); // 完成时进度置满
      }
    });
  },

  /** 安装完成后重启应用使新版本生效（进程立即重启，后续代码不会执行）。 */
  relaunch(): Promise<void> {
    return relaunch();
  },
};
