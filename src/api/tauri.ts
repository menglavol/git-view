// =====================================================================
// Tauri 调用统一封装
// 作用：集中处理 invoke 错误，把 Rust 端 GitViewError JSON 转换为
//       前端友好的 GitViewClientError；提供 listenEvent 简化订阅。
// 安全：错误对象不打印原始 detail 到 console，避免泄漏潜在凭据。
// =====================================================================

import { invoke, type InvokeArgs } from '@tauri-apps/api/core';
import { listen, type EventCallback, type UnlistenFn } from '@tauri-apps/api/event';

import { localizeError, type GitViewErrorCode } from './error-messages';

/**
 * 前端统一错误类型。
 *
 * 对应 Rust 端 `GitViewError` 序列化结构 `{ code, detail? }`。
 * 通过 `code` 字段可在 UI 层做差异化处理（如 Token 失效引导重新登录）。
 */
export class GitViewClientError extends Error {
  /** 错误代码（与 Rust GitViewError variant 名一致） */
  public readonly code: GitViewErrorCode;
  /** 后端附带的诊断信息（已脱敏，可空） */
  public readonly detail?: string;

  public constructor(code: GitViewErrorCode, detail?: string) {
    super(localizeError(code, detail));
    this.name = 'GitViewClientError';
    this.code = code;
    this.detail = detail;
  }
}

/**
 * 调用 Tauri 后端命令的统一封装。
 *
 * - 自动把 Rust 抛出的 `{ code, detail }` 错误转换为 `GitViewClientError`
 * - 调用方仅需 `try/catch` 一种错误类型
 * - 命令未注册或参数类型不匹配时，Tauri 自身抛出字符串，会被包装为 `Internal`
 *
 * @param cmd  Rust 端 `#[tauri::command]` 注册的命令名
 * @param args 命令参数，键名必须为 camelCase 以匹配 Tauri 2 默认约定
 */
export async function invokeCmd<T>(cmd: string, args?: InvokeArgs): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    // Rust GitViewError 序列化形态：{ code: 'TokenInvalid', detail?: '...' }
    if (typeof e === 'object' && e !== null && 'code' in e) {
      const obj = e as { code: string; detail?: string };
      throw new GitViewClientError(obj.code as GitViewErrorCode, obj.detail);
    }
    // 兜底：未知错误一律映射为 Internal，避免 UI 上裸露 JS 异常
    const message = e instanceof Error ? e.message : String(e);
    throw new GitViewClientError('Internal', message);
  }
}

/**
 * 订阅 Tauri 后端事件。
 *
 * @param event   事件名（与 Rust 端 `app.emit(event, payload)` 一致）
 * @param handler 收到事件 payload 时的回调
 * @returns       取消订阅函数，组件 unmount 时务必调用
 */
export async function listenEvent<T>(
  event: string,
  handler: (payload: T) => void,
): Promise<UnlistenFn> {
  const cb: EventCallback<T> = (e) => {
    handler(e.payload);
  };
  return listen<T>(event, cb);
}
