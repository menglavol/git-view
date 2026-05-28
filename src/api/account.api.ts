// =====================================================================
// 账号 API 封装
// 封装与 src-tauri/src/commands/accounts.rs 一一对应的 7 个 IPC 命令。
// 调用者无需关心 invoke 细节，只需处理强类型 Promise 与 GitViewClientError。
// =====================================================================

import { invokeCmd } from './tauri';
import type { Account, GitLabInstanceConfig, GitPlatform } from '@/types/account';

/** 自建 GitLab 实例配置 payload（嵌入在 AddAccountPayload 中）。 */
export interface AddGitLabInstanceConfigPayload {
  allowInsecureHttp?: boolean;
  allowInvalidCerts?: boolean;
  useSystemProxy?: boolean;
  proxyUrl?: string;
  requestTimeoutSeconds?: number;
  defaultCloneProtocol?: 'https' | 'ssh';
  sshHostAlias?: string;
  apiPathPrefix?: string;
}

/** 添加账号 payload。 */
export interface AddAccountPayload {
  platform: GitPlatform;
  webBaseUrl: string;
  /** 可空：后端按平台默认推导 */
  apiBaseUrl?: string;
  /** 用户提供的 Token 明文（保存到 keyring 后即可释放） */
  token: string;
  remark?: string;
  instanceConfig?: AddGitLabInstanceConfigPayload;
}

/** 测试连接 payload —— 与添加账号字段一致。 */
export type TestConnectionPayload = AddAccountPayload;

/** 测试连接响应（与后端 UserProfile 对齐）。 */
export interface UserProfile {
  username: string;
  displayName?: string;
  avatarUrl?: string;
}

/** 账号更新 payload（所有字段可选）。 */
export interface AccountUpdate {
  displayName?: string;
  avatarUrl?: string;
  remark?: string;
  /** FR-009：启用/禁用切换 */
  enabled?: boolean;
}

/**
 * 账号相关 API 集合。
 *
 * 所有方法均委托给 Tauri command；错误以 `GitViewClientError` 抛出，
 * 调用方可通过 `e.code` 区分（如 `TokenInvalid` / `BusyAccount`）。
 */
export const accountApi = {
  /** 添加账号（含连接测试 + 写入数据库 + 保存 keyring）。 */
  add(payload: AddAccountPayload): Promise<Account> {
    return invokeCmd<Account>('add_account', { payload });
  },

  /** 测试账号连接（不写入数据库）。 */
  test(payload: TestConnectionPayload): Promise<UserProfile> {
    return invokeCmd<UserProfile>('test_account_connection', { payload });
  },

  /** 列出所有账号（默认账号优先）。 */
  list(): Promise<Account[]> {
    return invokeCmd<Account[]>('list_accounts');
  },

  /** 更新账号字段。 */
  update(id: string, fields: AccountUpdate): Promise<Account> {
    return invokeCmd<Account>('update_account', { id, fields });
  },

  /** 删除账号（先清理 keyring 再删数据库）。 */
  delete(id: string): Promise<void> {
    return invokeCmd<void>('delete_account', { id });
  },

  /** 把指定账号设为默认。 */
  setDefault(id: string): Promise<void> {
    return invokeCmd<void>('set_default_account', { id });
  },

  /** 同步账号下的远程仓库（US1 阶段返回 0，US2 起返回实际数量）。 */
  syncRepositories(accountId: string): Promise<number> {
    return invokeCmd<number>('sync_account_repositories', { accountId });
  },
};

/** 类型重新导出，便于 store/components 引用。 */
export type { Account, GitLabInstanceConfig, GitPlatform };
