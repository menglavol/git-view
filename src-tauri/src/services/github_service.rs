//! GitHub Provider 实现。
//!
//! 调用 GitHub REST API v3（兼容 GitHub Enterprise）获取用户与仓库信息。
//!
//! 安全约束：
//!   - Token 字段为私有，禁止派生 `Debug`/`Display`
//!   - 所有错误消息生成前经 `redact_token` 脱敏

use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::{header, Client};
use serde::Deserialize;
use uuid::Uuid;

use crate::errors::{GitViewError, Result};
use crate::models::account::GitPlatform;
use crate::models::git::{CommitDetail, CommitFile, CommitFileStatus, CommitStats, CommitSummary};
use crate::models::repository::{RemoteRepository, Visibility};
use crate::services::provider::{
    parse_iso_datetime, truncate_file_diff, CommitPage, GitHostingProvider, RepositoryPage,
    UserProfile,
};
use crate::utils::redact::redact_token;

/// GitHub API 默认基址（公有云）。
const DEFAULT_API_BASE: &str = "https://api.github.com";

/// 请求超时（秒）。
const REQUEST_TIMEOUT_SECS: u64 = 30;

/// User-Agent 字符串（GitHub API 强制要求 UA Header）。
const USER_AGENT: &str = "GitView/1.0";

/// GitHub Provider。
///
/// 持有 token 与 HTTP Client；不实现 `Debug` 以防 token 泄露。
pub struct GitHubProvider {
    /// API 基址（含 scheme/host/可选端口）
    api_base_url: String,
    /// Personal Access Token —— 私有字段，绝不暴露
    token: String,
    /// 复用的 HTTP 客户端（连接池）
    client: Client,
}

impl GitHubProvider {
    /// 创建 GitHub Provider 实例。
    ///
    /// # Arguments
    ///
    /// * `api_base_url` - API 基址，留空则使用默认 `https://api.github.com`
    /// * `token` - 用户 PAT
    /// * `proxy` - 可选 HTTP/HTTPS 代理 URL
    pub fn new(api_base_url: Option<String>, token: String, proxy: Option<String>) -> Result<Self> {
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
            client,
        })
    }
}

/// GitHub `/user` 响应的最小字段子集。
#[derive(Debug, Deserialize)]
struct GitHubUserResp {
    login: String,
    name: Option<String>,
    avatar_url: Option<String>,
}

/// GitHub `/user/repos` 响应中单个仓库的字段子集。
#[derive(Debug, Deserialize)]
struct GitHubRepoResp {
    id: i64,
    full_name: String,
    name: String,
    owner: GitHubOwner,
    description: Option<String>,
    default_branch: String,
    private: bool,
    html_url: String,
    clone_url: String,
    ssh_url: Option<String>,
    #[allow(dead_code)]
    language: Option<String>,
    pushed_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubOwner {
    login: String,
}

/// GitHub 提交列表项字段子集。
#[derive(Debug, Deserialize)]
struct GitHubCommitListResp {
    sha: String,
    commit: GitHubCommitMeta,
    html_url: Option<String>,
}

/// GitHub 提交详情字段子集（含 stats 与 files）。
#[derive(Debug, Deserialize)]
struct GitHubCommitDetailResp {
    sha: String,
    commit: GitHubCommitMeta,
    html_url: Option<String>,
    #[serde(default)]
    parents: Vec<GitHubParent>,
    stats: Option<GitHubStats>,
    files: Option<Vec<GitHubFile>>,
}

/// 提交的 git 元信息（作者 / 提交者 / message）。
#[derive(Debug, Deserialize)]
struct GitHubCommitMeta {
    message: String,
    author: Option<GitHubGitActor>,
    committer: Option<GitHubGitActor>,
}

/// git 操作者（作者或提交者）的姓名 / 邮箱 / 时间。
#[derive(Debug, Deserialize)]
struct GitHubGitActor {
    name: Option<String>,
    email: Option<String>,
    date: Option<String>,
}

/// 父提交引用。
#[derive(Debug, Deserialize)]
struct GitHubParent {
    sha: String,
}

/// 提交增删行汇总。
#[derive(Debug, Deserialize)]
struct GitHubStats {
    additions: u32,
    deletions: u32,
    total: u32,
}

/// 单文件变更字段子集（patch 即 unified diff）。
#[derive(Debug, Deserialize)]
struct GitHubFile {
    filename: String,
    status: String,
    additions: Option<u32>,
    deletions: Option<u32>,
    patch: Option<String>,
    previous_filename: Option<String>,
}

#[async_trait]
impl GitHostingProvider for GitHubProvider {
    async fn get_current_user(&self) -> Result<UserProfile> {
        let url = format!("{}/user", self.api_base_url.trim_end_matches('/'));

        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await
            .map_err(|e| map_request_error("GitHub", &e))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(map_status_error("GitHub", status.as_u16()));
        }

        let user: GitHubUserResp = resp.json().await.map_err(|e| {
            GitViewError::ResponseDecode(format!(
                "解析 GitHub 用户响应失败：{}",
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
        let url = format!(
            "{}/user/repos?per_page={}&page={}&affiliation=owner,collaborator,organization_member&sort=pushed&direction=desc",
            self.api_base_url.trim_end_matches('/'),
            per_page,
            page,
        );

        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await
            .map_err(|e| map_request_error("GitHub", &e))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(map_status_error("GitHub", status.as_u16()));
        }

        let link_header = resp
            .headers()
            .get("link")
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        let repos: Vec<GitHubRepoResp> = resp.json().await.map_err(|e| {
            GitViewError::ResponseDecode(format!(
                "解析 GitHub 仓库列表失败：{}",
                redact_token(&e.to_string())
            ))
        })?;

        let has_next = link_header
            .as_deref()
            .is_some_and(|h| h.contains("rel=\"next\""));

        let now = Utc::now();
        let items: Vec<RemoteRepository> = repos
            .into_iter()
            .map(|r| {
                let pushed_at = r
                    .pushed_at
                    .as_deref()
                    .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.with_timezone(&Utc));
                RemoteRepository {
                    id: Uuid::new_v4().to_string(),
                    account_id: account_id.to_string(),
                    platform: GitPlatform::Github,
                    remote_id: r.id.to_string(),
                    full_name: r.full_name,
                    name: r.name,
                    owner: r.owner.login,
                    description: r.description,
                    visibility: if r.private {
                        Visibility::Private
                    } else {
                        Visibility::Public
                    },
                    default_branch: r.default_branch,
                    html_url: r.html_url,
                    ssh_url: r.ssh_url,
                    clone_url: r.clone_url,
                    is_favorite: false,
                    last_pushed_at: pushed_at,
                    synced_at: now,
                }
            })
            .collect();

        Ok(RepositoryPage { items, has_next })
    }

    async fn list_commits(
        &self,
        repo: &RemoteRepository,
        branch: Option<&str>,
        page: u32,
        per_page: u32,
    ) -> Result<CommitPage> {
        // branch 缺省回退仓库默认分支
        let sha = branch.unwrap_or(&repo.default_branch);
        let url = format!(
            "{}/repos/{}/{}/commits?sha={}&per_page={}&page={}",
            self.api_base_url.trim_end_matches('/'),
            repo.owner,
            repo.name,
            sha,
            per_page,
            page,
        );

        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await
            .map_err(|e| map_request_error("GitHub", &e))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(map_status_error("GitHub", status.as_u16()));
        }

        // Link 头含 rel="next" 表示还有下一页
        let has_next = resp
            .headers()
            .get("link")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|h| h.contains("rel=\"next\""));

        let list: Vec<GitHubCommitListResp> = resp.json().await.map_err(|e| {
            GitViewError::ResponseDecode(format!(
                "解析 GitHub 提交列表失败：{}",
                redact_token(&e.to_string())
            ))
        })?;

        let items = list
            .into_iter()
            .map(|c| {
                // 作者时间缺失时回退当前时间（GitHub 极少缺省）
                let authored_at = c
                    .commit
                    .author
                    .as_ref()
                    .and_then(|a| a.date.as_deref())
                    .and_then(parse_iso_datetime)
                    .unwrap_or_else(Utc::now);
                CommitSummary {
                    short_sha: c.sha.chars().take(7).collect(),
                    summary: c.commit.message.lines().next().unwrap_or("").to_string(),
                    author_name: c
                        .commit
                        .author
                        .as_ref()
                        .and_then(|a| a.name.clone())
                        .unwrap_or_default(),
                    authored_at,
                    html_url: c.html_url,
                    sha: c.sha,
                }
            })
            .collect();

        Ok(CommitPage { items, has_next })
    }

    async fn get_commit_detail(&self, repo: &RemoteRepository, sha: &str) -> Result<CommitDetail> {
        let url = format!(
            "{}/repos/{}/{}/commits/{}",
            self.api_base_url.trim_end_matches('/'),
            repo.owner,
            repo.name,
            sha,
        );

        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await
            .map_err(|e| map_request_error("GitHub", &e))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(map_status_error("GitHub", status.as_u16()));
        }

        let detail: GitHubCommitDetailResp = resp.json().await.map_err(|e| {
            GitViewError::ResponseDecode(format!(
                "解析 GitHub 提交详情失败：{}",
                redact_token(&e.to_string())
            ))
        })?;

        Ok(map_github_detail(detail))
    }
}

/// 把 GitHub 提交详情响应映射为统一的 `CommitDetail`。
fn map_github_detail(d: GitHubCommitDetailResp) -> CommitDetail {
    let author = d.commit.author;
    let committer = d.commit.committer;
    // 作者时间缺失回退当前时间；提交者时间可空
    let authored_at = author
        .as_ref()
        .and_then(|a| a.date.as_deref())
        .and_then(parse_iso_datetime)
        .unwrap_or_else(Utc::now);
    let committed_at = committer
        .as_ref()
        .and_then(|a| a.date.as_deref())
        .and_then(parse_iso_datetime);
    let files = d
        .files
        .unwrap_or_default()
        .into_iter()
        .map(map_github_file)
        .collect();
    CommitDetail {
        short_sha: d.sha.chars().take(7).collect(),
        sha: d.sha,
        message: d.commit.message,
        author_name: author
            .as_ref()
            .and_then(|a| a.name.clone())
            .unwrap_or_default(),
        author_email: author
            .as_ref()
            .and_then(|a| a.email.clone())
            .unwrap_or_default(),
        authored_at,
        committer_name: committer.as_ref().and_then(|a| a.name.clone()),
        committer_email: committer.and_then(|a| a.email),
        committed_at,
        parent_shas: d.parents.into_iter().map(|p| p.sha).collect(),
        html_url: d.html_url,
        stats: d.stats.map(|s| CommitStats {
            additions: s.additions,
            deletions: s.deletions,
            total: s.total,
        }),
        files,
    }
}

/// 把 GitHub 单文件变更映射为统一的 `CommitFile`（含截断后的 diff）。
fn map_github_file(f: GitHubFile) -> CommitFile {
    let status = match f.status.as_str() {
        "added" => CommitFileStatus::Added,
        "removed" => CommitFileStatus::Deleted,
        "renamed" => CommitFileStatus::Renamed,
        // modified / changed / copied 等统一归为修改
        _ => CommitFileStatus::Modified,
    };
    let (diff, truncated) = f.patch.map_or((None, false), |p| {
        let (text, tr) = truncate_file_diff(&p);
        (Some(text), tr)
    });
    CommitFile {
        path: f.filename,
        old_path: f.previous_filename,
        status,
        additions: f.additions,
        deletions: f.deletions,
        diff,
        truncated,
    }
}

/// 把 `reqwest::Error` 映射到 `GitViewError`（统一在 provider 层做平台前缀注入）。
fn map_request_error(platform: &str, e: &reqwest::Error) -> GitViewError {
    if e.is_timeout() {
        return GitViewError::Network(format!("{platform} 请求超时"));
    }
    if e.is_connect() {
        return GitViewError::Network(format!("{platform} 连接失败，请检查网络或代理"));
    }
    // 兜底：使用 reqwest 错误 Display + 脱敏
    GitViewError::Network(redact_token(&e.to_string()))
}

/// 把 HTTP 状态码映射到 `GitViewError`。
fn map_status_error(platform: &str, status: u16) -> GitViewError {
    match status {
        401 => GitViewError::TokenInvalid,
        403 => GitViewError::Forbidden,
        404 => GitViewError::NotFound(format!("{platform} 资源不存在")),
        _ => GitViewError::Network(format!("{platform} 请求失败，HTTP {status}")),
    }
}

// =====================================================================
// 单元测试
//
// 真实 HTTP 路径在 tests/integration/github_service_test.rs（T039）使用 wiremock
// 覆盖；本模块的单测仅断言不依赖网络的辅助逻辑。
// =====================================================================
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn status_error_mapping() {
        assert!(matches!(
            map_status_error("GitHub", 401),
            GitViewError::TokenInvalid
        ));
        assert!(matches!(
            map_status_error("GitHub", 403),
            GitViewError::Forbidden
        ));
        assert!(matches!(
            map_status_error("GitHub", 404),
            GitViewError::NotFound(_)
        ));
        assert!(matches!(
            map_status_error("GitHub", 500),
            GitViewError::Network(_)
        ));
    }

    #[test]
    fn constructor_with_default_base() {
        let provider = GitHubProvider::new(
            None,
            "ghp_test_token_only_unit_no_network".to_string(),
            None,
        )
        .expect("应能构造");
        assert_eq!(provider.api_base_url, DEFAULT_API_BASE);
    }

    #[test]
    fn constructor_with_proxy() {
        let provider = GitHubProvider::new(
            Some("https://api.github.com".to_string()),
            "ghp_token".to_string(),
            Some("http://localhost:8080".to_string()),
        );
        assert!(provider.is_ok());
    }
}
