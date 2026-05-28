//! Gitee Provider 实现。
//!
//! 调用 Gitee OpenAPI v5 获取用户与仓库信息。
//!
//! 认证支持两种模式：
//!   - Query 参数：`?access_token=<token>`（Gitee 文档主推方式）
//!   - Header：`Authorization: token <token>`（与 GitHub 旧版协议兼容）
//!
//! 本实现优先使用 Header 模式以避免 token 出现在 URL 中导致的日志泄露风险；
//! `get_current_user` 接口同时支持二者切换以便集成测试覆盖两种路径。

use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;
use uuid::Uuid;

use crate::errors::{GitViewError, Result};
use crate::models::account::GitPlatform;
use crate::models::repository::{RemoteRepository, Visibility};
use crate::services::provider::{GitHostingProvider, RepositoryPage, UserProfile};
use crate::utils::redact::redact_token;

/// Gitee API 默认基址。
const DEFAULT_API_BASE: &str = "https://gitee.com/api/v5";

const REQUEST_TIMEOUT_SECS: u64 = 30;
const USER_AGENT: &str = "GitView/1.0";

/// Gitee 认证模式。
///
/// 默认使用 `Header`：把 token 放在 Authorization 头，避免 URL 日志泄露。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GiteeAuthMode {
    /// Header `Authorization: token <token>`
    #[default]
    Header,
    /// Query 参数 `?access_token=<token>`
    Query,
}

/// Gitee Provider。
pub struct GiteeProvider {
    api_base_url: String,
    /// 私有 token —— 严禁暴露
    token: String,
    /// 认证模式
    auth_mode: GiteeAuthMode,
    client: Client,
}

impl GiteeProvider {
    /// 创建 Gitee Provider 实例。
    pub fn new(
        api_base_url: Option<String>,
        token: String,
        proxy: Option<String>,
        auth_mode: GiteeAuthMode,
    ) -> Result<Self> {
        let mut builder = Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS));

        if let Some(proxy_url) = proxy {
            let proxy = reqwest::Proxy::all(&proxy_url).map_err(|e| {
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
            api_base_url: api_base_url.unwrap_or_else(|| DEFAULT_API_BASE.to_string()),
            token,
            auth_mode,
            client,
        })
    }
}

/// Gitee `/user` 响应的最小字段子集。
#[derive(Debug, Deserialize)]
struct GiteeUserResp {
    login: String,
    name: Option<String>,
    avatar_url: Option<String>,
}

/// Gitee `/user/repos` 响应中单个仓库的字段子集。
#[derive(Debug, Deserialize)]
struct GiteeRepoResp {
    id: i64,
    full_name: String,
    name: String,
    owner: GiteeOwner,
    description: Option<String>,
    default_branch: Option<String>,
    #[serde(rename = "private")]
    is_private: bool,
    html_url: String,
    ssh_url: Option<String>,
    #[serde(rename = "clone_url")]
    https_url: Option<String>,
    pushed_at: Option<String>,
    #[allow(dead_code)]
    is_public: Option<bool>,
    #[serde(rename = "internal")]
    is_internal: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct GiteeOwner {
    login: String,
}

#[async_trait]
impl GitHostingProvider for GiteeProvider {
    async fn get_current_user(&self) -> Result<UserProfile> {
        let base = format!("{}/user", self.api_base_url.trim_end_matches('/'));

        let req = match self.auth_mode {
            GiteeAuthMode::Header => self
                .client
                .get(&base)
                .header("Authorization", format!("token {}", self.token)),
            GiteeAuthMode::Query => self
                .client
                .get(&base)
                .query(&[("access_token", &self.token)]),
        };

        let resp = req
            .send()
            .await
            .map_err(|e| map_request_error("Gitee", &e))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(map_status_error("Gitee", status.as_u16()));
        }

        let user: GiteeUserResp = resp.json().await.map_err(|e| {
            GitViewError::Network(format!(
                "解析 Gitee 用户响应失败：{}",
                redact_token(&e.to_string())
            ))
        })?;

        Ok(UserProfile {
            username: user.login,
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
        let base = format!(
            "{}/user/repos?type=all&per_page={}&page={}&sort=pushed&direction=desc",
            self.api_base_url.trim_end_matches('/'),
            per_page,
            page,
        );

        let req = match self.auth_mode {
            GiteeAuthMode::Header => self
                .client
                .get(&base)
                .header("Authorization", format!("token {}", self.token)),
            GiteeAuthMode::Query => self
                .client
                .get(&base)
                .query(&[("access_token", &self.token)]),
        };

        let resp = req
            .send()
            .await
            .map_err(|e| map_request_error("Gitee", &e))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(map_status_error("Gitee", status.as_u16()));
        }

        let total_page = resp
            .headers()
            .get("total_page")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u32>().ok());

        let repos: Vec<GiteeRepoResp> = resp.json().await.map_err(|e| {
            GitViewError::Network(format!(
                "解析 Gitee 仓库列表失败：{}",
                redact_token(&e.to_string())
            ))
        })?;

        let has_next = total_page.is_some_and(|tp| page < tp) || repos.len() == per_page as usize;
        let now = Utc::now();

        let items: Vec<RemoteRepository> = repos
            .into_iter()
            .map(|r| {
                let pushed_at = r
                    .pushed_at
                    .as_deref()
                    .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.with_timezone(&Utc));
                let visibility = if r.is_private {
                    Visibility::Private
                } else if r.is_internal.unwrap_or(false) {
                    Visibility::Internal
                } else {
                    Visibility::Public
                };
                let clone_url = r.https_url.unwrap_or_else(|| r.html_url.clone() + ".git");
                RemoteRepository {
                    id: Uuid::new_v4().to_string(),
                    account_id: account_id.to_string(),
                    platform: GitPlatform::Gitee,
                    remote_id: r.id.to_string(),
                    full_name: r.full_name,
                    name: r.name,
                    owner: r.owner.login,
                    description: r.description,
                    visibility,
                    default_branch: r.default_branch.unwrap_or_else(|| "master".to_string()),
                    html_url: r.html_url,
                    ssh_url: r.ssh_url,
                    clone_url,
                    is_favorite: false,
                    last_pushed_at: pushed_at,
                    synced_at: now,
                }
            })
            .collect();

        Ok(RepositoryPage { items, has_next })
    }
}

fn map_request_error(platform: &str, e: &reqwest::Error) -> GitViewError {
    if e.is_timeout() {
        return GitViewError::Network(format!("{platform} 请求超时"));
    }
    if e.is_connect() {
        return GitViewError::Network(format!("{platform} 连接失败，请检查网络或代理"));
    }
    GitViewError::Network(redact_token(&e.to_string()))
}

fn map_status_error(platform: &str, status: u16) -> GitViewError {
    match status {
        401 => GitViewError::TokenInvalid,
        403 => GitViewError::Forbidden,
        404 => GitViewError::NotFound(format!("{platform} 资源不存在")),
        _ => GitViewError::Network(format!("{platform} 请求失败，HTTP {status}")),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn status_error_mapping() {
        assert!(matches!(
            map_status_error("Gitee", 401),
            GitViewError::TokenInvalid
        ));
        assert!(matches!(
            map_status_error("Gitee", 403),
            GitViewError::Forbidden
        ));
    }

    #[test]
    fn default_auth_mode_is_header() {
        assert_eq!(GiteeAuthMode::default(), GiteeAuthMode::Header);
    }

    #[test]
    fn constructor_defaults_api_base() {
        let provider = GiteeProvider::new(
            None,
            "gitee_test_token".to_string(),
            None,
            GiteeAuthMode::Header,
        )
        .expect("应能构造");
        assert_eq!(provider.api_base_url, DEFAULT_API_BASE);
    }
}
