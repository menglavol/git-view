//! Git 托管平台 Provider 抽象。
//!
//! 通过一个共同的 trait 统一三大平台（GitHub / GitLab / Gitee）的接口，
//! 上层 `account_service` 无需关心具体平台差异。
//!
//! 各平台实现位于：
//!   - [`crate::services::github_service`]
//!   - [`crate::services::gitlab_service`]
//!   - [`crate::services::gitee_service`]
//!
//! 安全约束（宪法 Principle III）：
//!   - Provider 实现的构造函数接收 token 后，必须将其存储为不可序列化字段
//!   - Provider 任何 `Debug`/`Display` 输出均不得包含 token
//!   - 错误消息通过 `crate::utils::redact::redact_token` 脱敏后再向上传递

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::errors::Result;
use crate::models::repository::RemoteRepository;

/// 平台用户档案（连接测试与账号同步用）。
///
/// 跨平台统一字段集合 —— 各 Provider 负责把 API 响应映射到本结构。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserProfile {
    /// 平台用户名（如 `octocat`）
    pub username: String,
    /// 用户显示名（GitHub `name` / GitLab `name` / Gitee `name`）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// 头像 URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
}

/// 仓库分页结果（list_repositories 使用，US2 起填充实际数据）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryPage {
    /// 当前页仓库列表
    pub items: Vec<RemoteRepository>,
    /// 是否还有下一页（用于前端"加载更多"判断）
    pub has_next: bool,
}

/// Git 托管平台 Provider 抽象。
///
/// 实现者负责：
///   - 持有该账号的 token 与 HTTP Client 配置
///   - 把 HTTP 错误映射为 `GitViewError` 的对应变体
///   - 把平台 API 响应映射到统一模型
///
/// `Send + Sync` 约束允许 Provider 在 `Arc` 共享下跨线程使用，
/// 满足异步任务并发调用。
#[async_trait]
pub trait GitHostingProvider: Send + Sync {
    /// 获取当前 token 对应的用户档案。
    ///
    /// 用于"测试连接"按钮与账号添加流程的身份确认。
    async fn get_current_user(&self) -> Result<UserProfile>;

    /// 拉取当前账号的仓库列表（分页）。
    ///
    /// `account_id` 用于把返回的 `RemoteRepository` 关联到本地数据库账号行；
    /// Provider 实现负责把平台 API 响应映射为统一的 `RemoteRepository` 结构。
    ///
    /// 默认实现返回空页，便于尚未实现该方法的 Provider 编译通过。
    async fn list_repositories(
        &self,
        _page: u32,
        _per_page: u32,
        _account_id: &str,
    ) -> Result<RepositoryPage> {
        Ok(RepositoryPage {
            items: Vec::new(),
            has_next: false,
        })
    }
}
