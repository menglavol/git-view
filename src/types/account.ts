// =====================================================================
// 账号相关类型定义
// 与 src-tauri/src/models/account.rs 一一对应，字段名 camelCase。
// =====================================================================

/** Git 托管平台字面量联合类型（与 Rust GitPlatform 枚举对齐）。 */
export type GitPlatform = 'github' | 'gitlab' | 'gitee';

/**
 * 账号实体。
 *
 * 不含 token 明文字段；token 仅通过 `tokenKey` 引用系统密钥库中的条目。
 */
export interface Account {
  id: string;
  platform: GitPlatform;
  webBaseUrl: string;
  apiBaseUrl: string;
  username: string;
  displayName?: string;
  avatarUrl?: string;
  tokenKey: string;
  isDefault: boolean;
  /** FR-009：账号启用状态；禁用后不参与同步与列表展示 */
  enabled: boolean;
  /** 用户备注，可空 */
  remark?: string;
  /** ISO 8601 字符串 */
  createdAt: string;
  updatedAt: string;
  lastSyncAt?: string;
}

/** 实例默认 Clone 协议偏好。 */
export type CloneProtocolPref = 'https' | 'ssh';

/** 连接测试状态。 */
export type ConnectionStatus = 'unknown' | 'success' | 'failed';

/** 自建 GitLab 实例配置（扩展字段集合）。 */
export interface GitLabInstanceConfig {
  id: string;
  accountId: string;
  webBaseUrl: string;
  apiBaseUrl: string;
  allowInsecureHttp: boolean;
  allowInvalidCerts: boolean;
  useSystemProxy: boolean;
  proxyUrl?: string;
  requestTimeoutSeconds?: number;
  defaultCloneProtocol: CloneProtocolPref;
  sshHostAlias?: string;
  apiPathPrefix?: string;
  lastConnectionStatus: ConnectionStatus;
  lastConnectionError?: string;
  createdAt: string;
  updatedAt: string;
}
