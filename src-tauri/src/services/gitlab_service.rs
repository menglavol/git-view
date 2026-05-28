//! GitLab Provider 实现。
//!
//! 支持 gitlab.com 与自建 GitLab 实例：
//!   - 自签名证书白名单：仅对该实例生效，不污染全局 TLS 配置
//!   - 自定义代理：实例级独立配置
//!   - PRIVATE-TOKEN 头认证
//!
//! 同时提供 `derive_gitlab_api_url`：从 Web 地址自动推导 API 基址（T029）。

use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;
use url::Url;
use uuid::Uuid;

use crate::errors::{GitViewError, Result};
use crate::models::account::GitPlatform;
use crate::models::repository::{RemoteRepository, Visibility};
use crate::services::provider::{GitHostingProvider, RepositoryPage, UserProfile};
use crate::utils::redact::redact_token;

/// 请求超时（秒）。
const REQUEST_TIMEOUT_SECS: u64 = 30;

/// User-Agent。
const USER_AGENT: &str = "GitView/1.0";

/// GitLab Provider 构造时所需的实例级配置。
///
/// 与 `models::account::GitLabInstanceConfig` 解耦：本结构仅承载 Provider 所需
/// 运行时配置（不含数据库字段如 `id` / `created_at`）。`account_service`
/// 在实例化 Provider 前完成模型 → 配置的转换。
#[derive(Debug, Clone)]
pub struct GitLabClientConfig {
    /// API 基址（含 scheme/host/可选端口/可选子路径）
    pub api_base_url: String,
    /// 是否允许自签名 / 无效证书（仅对该实例生效）
    pub allow_invalid_certs: bool,
    /// 可选代理 URL
    pub proxy_url: Option<String>,
    /// 请求超时（秒），未提供时使用默认 30s
    pub request_timeout_seconds: Option<u64>,
}

/// GitLab Provider。
pub struct GitLabProvider {
    api_base_url: String,
    /// 私有 token —— 严禁暴露于 Debug 等输出
    token: String,
    client: Client,
}

impl GitLabProvider {
    /// 创建 GitLab Provider 实例。
    pub fn new(config: GitLabClientConfig, token: String) -> Result<Self> {
        let timeout = config
            .request_timeout_seconds
            .unwrap_or(REQUEST_TIMEOUT_SECS);

        let mut builder = Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(timeout));

        if config.allow_invalid_certs {
            // 仅在本实例 Client 上启用，不影响全局
            builder = builder.danger_accept_invalid_certs(true);
        }

        if let Some(proxy_url) = config.proxy_url.as_deref() {
            let proxy = reqwest::Proxy::all(proxy_url).map_err(|e| {
                GitViewError::Network(format!("代理地址无效：{}", redact_token(&e.to_string())))
            })?;
            builder = builder.proxy(proxy);
        }

        let client = builder.build().map_err(|e| {
            GitViewError::Internal(format!(
                "HTTP 客户端初始化失败：{}",
                redact_token(&e.to_string())
            ))
        })?;

        Ok(Self {
            api_base_url: config.api_base_url,
            token,
            client,
        })
    }
}

/// GitLab `/user` 响应的最小字段子集。
#[derive(Debug, Deserialize)]
struct GitLabUserResp {
    /// 平台内部数值 ID（保留以便后续 ID 关联）
    #[allow(dead_code)]
    id: i64,
    username: String,
    name: Option<String>,
    avatar_url: Option<String>,
    /// 用户主页 URL（保留供 UI 跳转使用）
    #[allow(dead_code)]
    web_url: Option<String>,
}

/// GitLab `/projects` 响应中单个项目的字段子集。
#[derive(Debug, Deserialize)]
struct GitLabProjectResp {
    id: i64,
    path_with_namespace: String,
    name: String,
    namespace: GitLabNamespace,
    description: Option<String>,
    default_branch: Option<String>,
    visibility: String,
    web_url: String,
    http_url_to_repo: String,
    ssh_url_to_repo: Option<String>,
    last_activity_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitLabNamespace {
    path: String,
}

#[async_trait]
impl GitHostingProvider for GitLabProvider {
    async fn get_current_user(&self) -> Result<UserProfile> {
        let url = format!("{}/user", self.api_base_url.trim_end_matches('/'));

        let resp = self
            .client
            .get(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .send()
            .await
            .map_err(|e| map_request_error("GitLab", &e))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(map_status_error("GitLab", status.as_u16()));
        }

        let user: GitLabUserResp = resp.json().await.map_err(|e| {
            GitViewError::Network(format!(
                "解析 GitLab 用户响应失败：{}",
                redact_token(&e.to_string())
            ))
        })?;

        Ok(UserProfile {
            username: user.username,
            display_name: user.name,
            avatar_url: user.avatar_url,
        })
    }

    async fn list_repositories(
        &self,
        page: u32,
        per_page: u32,
        account_id: &str,
    ) -> Result<RepositoryPage> {
        let url = format!(
            "{}/projects?membership=true&simple=true&per_page={}&page={}&order_by=last_activity_at&sort=desc",
            self.api_base_url.trim_end_matches('/'),
            per_page,
            page,
        );

        let resp = self
            .client
            .get(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .send()
            .await
            .map_err(|e| map_request_error("GitLab", &e))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(map_status_error("GitLab", status.as_u16()));
        }

        let next_page = resp
            .headers()
            .get("x-next-page")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u32>().ok());

        let projects: Vec<GitLabProjectResp> = resp.json().await.map_err(|e| {
            GitViewError::Network(format!(
                "解析 GitLab 项目列表失败：{}",
                redact_token(&e.to_string())
            ))
        })?;

        let has_next = next_page.is_some_and(|n| n > 0);
        let now = Utc::now();

        let items: Vec<RemoteRepository> = projects
            .into_iter()
            .map(|p| {
                let pushed_at = p
                    .last_activity_at
                    .as_deref()
                    .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.with_timezone(&Utc));
                let visibility = match p.visibility.as_str() {
                    "public" => Visibility::Public,
                    "internal" => Visibility::Internal,
                    _ => Visibility::Private,
                };
                RemoteRepository {
                    id: Uuid::new_v4().to_string(),
                    account_id: account_id.to_string(),
                    platform: GitPlatform::Gitlab,
                    remote_id: p.id.to_string(),
                    full_name: p.path_with_namespace,
                    name: p.name,
                    owner: p.namespace.path,
                    description: p.description,
                    visibility,
                    default_branch: p.default_branch.unwrap_or_else(|| "main".to_string()),
                    html_url: p.web_url,
                    ssh_url: p.ssh_url_to_repo,
                    clone_url: p.http_url_to_repo,
                    is_favorite: false,
                    last_pushed_at: pushed_at,
                    synced_at: now,
                }
            })
            .collect();

        Ok(RepositoryPage { items, has_next })
    }
}

// =====================================================================
// API URL 推导（T029）
// =====================================================================

/// 根据 GitLab Web 地址推导 API 地址。
///
/// 规则：
///   - 保留 scheme、host、端口
///   - 在已有路径末尾追加 `/api/v4`（子路径部署时如 `https://x/gitlab` →
///     `https://x/gitlab/api/v4`）
///   - 末尾 `/` 规范化（避免出现双斜杠）
///
/// # Examples
///
/// ```
/// use gitview_lib::services::gitlab_service::derive_gitlab_api_url;
/// assert_eq!(
///     derive_gitlab_api_url("https://gitlab.com").unwrap(),
///     "https://gitlab.com/api/v4"
/// );
/// assert_eq!(
///     derive_gitlab_api_url("https://code.company.com/gitlab/").unwrap(),
///     "https://code.company.com/gitlab/api/v4"
/// );
/// ```
pub fn derive_gitlab_api_url(web_url: &str) -> Result<String> {
    let url = Url::parse(web_url)
        .map_err(|e| GitViewError::ApiUrlInvalid(format!("Web 地址无法解析：{e}")))?;

    // 校验 scheme（仅允许 http/https）
    if !matches!(url.scheme(), "http" | "https") {
        return Err(GitViewError::ApiUrlInvalid(format!(
            "仅支持 http/https，实际：{}",
            url.scheme()
        )));
    }

    if url.host_str().is_none() {
        return Err(GitViewError::ApiUrlInvalid("缺少 host".to_string()));
    }

    // 组装：scheme://host:port/<existing-path>/api/v4
    let mut base = format!("{}://{}", url.scheme(), url.host_str().unwrap_or(""));
    if let Some(port) = url.port() {
        base.push(':');
        base.push_str(&port.to_string());
    }

    // 已有路径（去掉首尾斜杠后再统一拼接）
    let path = url.path().trim_matches('/');
    if path.is_empty() {
        Ok(format!("{base}/api/v4"))
    } else {
        Ok(format!("{base}/{path}/api/v4"))
    }
}

// =====================================================================
// 错误映射（与 GitHub Provider 共用语义，但保留独立函数便于平台前缀注入）
// =====================================================================

fn map_request_error(platform: &str, e: &reqwest::Error) -> GitViewError {
    if e.is_timeout() {
        return GitViewError::Network(format!("{platform} 请求超时"));
    }
    if e.is_connect() {
        return GitViewError::Network(format!("{platform} 连接失败，请检查网络或代理"));
    }
    // TLS 错误专项识别（GitLab 自签名场景特别相关）
    let msg = e.to_string();
    if msg.contains("certificate") || msg.contains("TLS") || msg.contains("tls") {
        return GitViewError::TlsCert(redact_token(&msg));
    }
    GitViewError::Network(redact_token(&msg))
}

fn map_status_error(platform: &str, status: u16) -> GitViewError {
    match status {
        401 => GitViewError::TokenInvalid,
        // GitLab 会用 403 表示 Token 被撤销，与 GitHub 的 401 语义不完全一致 —— 这里仍保留 Forbidden 语义，
        // 上层 UI 可结合错误细节给出"请重新生成 Token"提示
        403 => GitViewError::Forbidden,
        404 => GitViewError::NotFound(format!("{platform} 资源不存在")),
        _ => GitViewError::Network(format!("{platform} 请求失败，HTTP {status}")),
    }
}

// =====================================================================
// 单元测试
// =====================================================================
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn derive_url_standard_gitlab_com() {
        assert_eq!(
            derive_gitlab_api_url("https://gitlab.com").unwrap(),
            "https://gitlab.com/api/v4"
        );
        assert_eq!(
            derive_gitlab_api_url("https://gitlab.com/").unwrap(),
            "https://gitlab.com/api/v4"
        );
    }

    #[test]
    fn derive_url_custom_port() {
        assert_eq!(
            derive_gitlab_api_url("https://gitlab.company.com:8443").unwrap(),
            "https://gitlab.company.com:8443/api/v4"
        );
    }

    #[test]
    fn derive_url_http_intranet() {
        assert_eq!(
            derive_gitlab_api_url("http://10.0.0.5").unwrap(),
            "http://10.0.0.5/api/v4"
        );
    }

    #[test]
    fn derive_url_subpath_deployment() {
        assert_eq!(
            derive_gitlab_api_url("https://code.company.com/gitlab").unwrap(),
            "https://code.company.com/gitlab/api/v4"
        );
        assert_eq!(
            derive_gitlab_api_url("https://code.company.com/gitlab/").unwrap(),
            "https://code.company.com/gitlab/api/v4"
        );
    }

    #[test]
    fn derive_url_rejects_invalid_scheme() {
        let err = derive_gitlab_api_url("ftp://gitlab.com").unwrap_err();
        assert!(matches!(err, GitViewError::ApiUrlInvalid(_)));
    }

    #[test]
    fn derive_url_rejects_unparseable() {
        let err = derive_gitlab_api_url("not a url").unwrap_err();
        assert!(matches!(err, GitViewError::ApiUrlInvalid(_)));
    }

    #[test]
    fn status_error_mapping() {
        assert!(matches!(
            map_status_error("GitLab", 401),
            GitViewError::TokenInvalid
        ));
        assert!(matches!(
            map_status_error("GitLab", 403),
            GitViewError::Forbidden
        ));
    }

    #[test]
    fn provider_constructor_with_self_signed() {
        let cfg = GitLabClientConfig {
            api_base_url: "https://gitlab.example.com/api/v4".to_string(),
            allow_invalid_certs: true,
            proxy_url: None,
            request_timeout_seconds: Some(10),
        };
        let provider = GitLabProvider::new(cfg, "glpat-test_token_for_unit".to_string());
        assert!(provider.is_ok());
    }
}
