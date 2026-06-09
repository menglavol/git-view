//! 账号领域模型。
//!
//! 描述用户在 GitView 中配置的 Git 托管平台账号：
//!   - 公有云平台：GitHub / Gitee（host 固定）
//!   - 自建实例：GitLab（host 可自定义，需配合 `GitLabInstanceConfig`）
//!
//! 安全约束：本结构体不持有 token 明文，凭据通过 `token_key` 关联到
//! 系统密钥库（macOS Keychain / Windows Credential Manager / Linux Secret Service）。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 支持的 Git 托管平台枚举。
///
/// `snake_case` 序列化形式与前端 `type GitPlatform = 'github' | 'gitlab' | 'gitee'`
/// 字面量联合类型保持一致。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitPlatform {
    /// GitHub.com（公有云）
    Github,
    /// GitLab.com 或自建 GitLab 实例（需配合 `web_base_url` 与 `api_base_url`）
    Gitlab,
    /// Gitee.com（公有云）
    Gitee,
}

/// 账号实体。
///
/// 字段命名与数据库表 `accounts` 列名一致（snake_case 存储 / serde camelCase 输出）。
/// 必含 `enabled: bool` 对应 FR-009 启用/禁用账号功能。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    /// 账号唯一标识（UUID v4）
    pub id: String,
    /// 所属平台
    pub platform: GitPlatform,
    /// 网页基础 URL，如 `https://github.com` 或 `https://gitlab.example.com`
    pub web_base_url: String,
    /// API 基础 URL，如 `https://api.github.com` 或 `https://gitlab.example.com/api/v4`
    pub api_base_url: String,
    /// 平台用户名（如 `octocat`）
    pub username: String,
    /// 用户显示名（可空）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// 头像 URL（可空）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    /// 凭据在系统密钥库中的键（`gitview:<account_id>`），不含 token 明文
    pub token_key: String,
    /// 是否为默认账号（同时只允许一个账号 `is_default = true`）
    pub is_default: bool,
    /// 是否启用（FR-009：禁用后该账号不参与同步与列表展示）
    pub enabled: bool,
    /// 默认 Clone 协议（https / ssh）——账户级，决定批量 clone 走 SSH 还是 HTTPS
    pub default_clone_protocol: CloneProtocolPref,
    /// 用户备注（可空），供 UI 自定义展示
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remark: Option<String>,
    /// 记录创建时间
    pub created_at: DateTime<Utc>,
    /// 记录最近更新时间
    pub updated_at: DateTime<Utc>,
    /// 最近一次成功同步远程仓库的时间（可空）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_sync_at: Option<DateTime<Utc>>,
}

/// 自建 GitLab 实例配置。
///
/// 当 `platform == GitPlatform::Gitlab` 且 `web_base_url` 非 `gitlab.com` 时，
/// 该结构体记录额外的实例级配置（TLS / 代理 / 协议 / 路径前缀等）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitLabInstanceConfig {
    /// 实例唯一标识（与所属账号一对一）
    pub id: String,
    /// 所属账号 ID（外键到 accounts.id）
    pub account_id: String,
    /// 实例网页基础 URL
    pub web_base_url: String,
    /// 实例 API 基础 URL（自动推导，允许用户覆盖）
    pub api_base_url: String,
    /// 是否允许 HTTP（非 HTTPS），用户需显式确认
    pub allow_insecure_http: bool,
    /// 是否允许自签名/无效证书（仅对该实例生效，不污染全局 TLS 配置）
    pub allow_invalid_certs: bool,
    /// 是否使用系统代理（若 false 则使用 `proxy_url`）
    pub use_system_proxy: bool,
    /// 自定义代理 URL（可空）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_url: Option<String>,
    /// 请求超时（秒），未配置时使用默认值
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_timeout_seconds: Option<u64>,
    /// 默认 Clone 协议（https / ssh）
    pub default_clone_protocol: CloneProtocolPref,
    /// SSH 主机别名（用于 `git@<alias>:<owner>/<repo>.git`）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_host_alias: Option<String>,
    /// API 路径前缀（如 `/api/v4`），通常由 URL 推导自动填充
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_path_prefix: Option<String>,
    /// 最近一次连接测试状态（success / failed / unknown）
    pub last_connection_status: ConnectionStatus,
    /// 最近一次连接测试失败时的简短错误（已脱敏）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_connection_error: Option<String>,
    /// 记录创建时间
    pub created_at: DateTime<Utc>,
    /// 记录最近更新时间
    pub updated_at: DateTime<Utc>,
}

/// 实例默认 Clone 协议偏好。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CloneProtocolPref {
    /// HTTPS（需要 Token / 凭据助手）
    #[default]
    Https,
    /// SSH（需要本地 SSH key）
    Ssh,
}

/// 连接测试状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionStatus {
    /// 从未测试
    #[default]
    Unknown,
    /// 最近一次测试成功
    Success,
    /// 最近一次测试失败
    Failed,
}

// =====================================================================
// DTO（前后端请求/响应载荷）
// =====================================================================

/// 添加 GitLab 实例配置 payload（嵌入在 `AddAccountPayload` 中）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddGitLabInstanceConfigPayload {
    /// 是否允许 HTTP（用户在前端勾选时需弹窗确认）
    #[serde(default)]
    pub allow_insecure_http: bool,
    /// 是否允许自签名/无效证书
    #[serde(default)]
    pub allow_invalid_certs: bool,
    /// 是否使用系统代理
    #[serde(default = "default_true")]
    pub use_system_proxy: bool,
    /// 自定义代理 URL（可空）
    #[serde(default)]
    pub proxy_url: Option<String>,
    /// 请求超时（秒）
    #[serde(default)]
    pub request_timeout_seconds: Option<u64>,
    /// 默认 Clone 协议
    #[serde(default)]
    pub default_clone_protocol: CloneProtocolPref,
    /// SSH 主机别名（可空）
    #[serde(default)]
    pub ssh_host_alias: Option<String>,
    /// API 路径前缀（可空）
    #[serde(default)]
    pub api_path_prefix: Option<String>,
}

const fn default_true() -> bool {
    true
}

/// 添加账号 payload。
///
/// 前端"添加账号"表单提交后由 Tauri command 反序列化为本结构。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddAccountPayload {
    /// 平台
    pub platform: GitPlatform,
    /// Web 地址（GitHub/Gitee 可省略并使用默认；GitLab 必填）
    pub web_base_url: String,
    /// API 地址（可空，留空时由 service 端推导）
    #[serde(default)]
    pub api_base_url: Option<String>,
    /// 用户填写的 token 明文（保存到 keyring 后即在内存清零）
    pub token: String,
    /// 备注（可空）
    #[serde(default)]
    pub remark: Option<String>,
    /// 默认 Clone 协议（账户级，所有平台通用）。前端表单新建默认 SSH；
    /// 缺省（旧前端不传）时回退枚举默认值 Https。
    #[serde(default)]
    pub default_clone_protocol: CloneProtocolPref,
    /// 自建 GitLab 实例配置（仅 GitLab 平台需要）
    #[serde(default)]
    pub instance_config: Option<AddGitLabInstanceConfigPayload>,
}

/// 测试连接 payload —— 与添加账号字段一致但不写入数据库。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestConnectionPayload {
    pub platform: GitPlatform,
    pub web_base_url: String,
    #[serde(default)]
    pub api_base_url: Option<String>,
    pub token: String,
    #[serde(default)]
    pub instance_config: Option<AddGitLabInstanceConfigPayload>,
}

/// 账号更新载荷（部分字段更新）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountUpdate {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub remark: Option<String>,
    /// 启用/禁用切换（FR-009）
    #[serde(default)]
    pub enabled: Option<bool>,
    /// 默认 Clone 协议（None 表示不修改）
    #[serde(default)]
    pub default_clone_protocol: Option<CloneProtocolPref>,
}

// =====================================================================
// 单元测试 —— 验证 JSON 序列化形态符合前后端契约
// =====================================================================
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    /// GitPlatform 序列化为 snake_case 字符串
    #[test]
    fn git_platform_serialize_snake_case() {
        let json = serde_json::to_string(&GitPlatform::Github).unwrap();
        assert_eq!(json, "\"github\"");
        let json = serde_json::to_string(&GitPlatform::Gitlab).unwrap();
        assert_eq!(json, "\"gitlab\"");
    }

    /// Account 字段使用 camelCase 序列化
    #[test]
    fn account_serialize_camel_case() {
        let now = Utc::now();
        let account = Account {
            id: "id-1".to_string(),
            platform: GitPlatform::Github,
            web_base_url: "https://github.com".to_string(),
            api_base_url: "https://api.github.com".to_string(),
            username: "octocat".to_string(),
            display_name: None,
            avatar_url: None,
            token_key: "gitview:id-1".to_string(),
            is_default: true,
            enabled: true,
            default_clone_protocol: CloneProtocolPref::Https,
            remark: None,
            created_at: now,
            updated_at: now,
            last_sync_at: None,
        };
        let json = serde_json::to_string(&account).unwrap();
        assert!(json.contains("\"webBaseUrl\""));
        assert!(json.contains("\"tokenKey\""));
        assert!(json.contains("\"isDefault\":true"));
        assert!(json.contains("\"enabled\":true"));
        // token 字段不应存在
        assert!(!json.contains("\"token\""));
    }
}
