//! 仓库领域模型。
//!
//! 区分两类仓库：
//!   - `RemoteRepository`：从托管平台拉取的远程仓库元数据（GitHub Repo / GitLab Project / Gitee Repo）
//!   - `LocalRepository`：用户已克隆到本地的 Git 仓库（与 `RemoteRepository` 通过 `remote_url` 关联）

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::models::account::GitPlatform;

/// 仓库可见性。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    /// 公开仓库
    Public,
    /// 私有仓库
    Private,
    /// 内部仓库（GitLab 特有）
    Internal,
}

/// 本地仓库状态（脏检测、未推送提交等汇总）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepositoryStatus {
    /// 工作区干净，与远端一致
    Clean,
    /// 工作区有未提交变更
    Dirty,
    /// 本地有提交未推送
    Ahead,
    /// 远端有提交未拉取
    Behind,
    /// 本地与远端均有未同步提交
    Diverged,
    /// 状态未知（如未关联远端）
    Unknown,
}

/// 远程仓库元数据。
///
/// 字段与 GitHub / GitLab / Gitee API 返回的字段尽可能对齐，
/// service 层负责把各平台 API 响应统一映射到本结构。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteRepository {
    /// 本地数据库主键
    pub id: String,
    /// 所属账号 ID
    pub account_id: String,
    /// 所属平台（冗余字段，避免 join 即可展示）
    pub platform: GitPlatform,
    /// 平台原生 ID（GitHub repo id / GitLab project id / Gitee repo id）
    pub remote_id: String,
    /// 仓库全名，如 `octocat/Hello-World`
    pub full_name: String,
    /// 仓库简称
    pub name: String,
    /// 所有者用户名 / 组织名
    pub owner: String,
    /// 仓库描述（可空）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 可见性
    pub visibility: Visibility,
    /// 默认分支名
    pub default_branch: String,
    /// 仓库网页 URL
    pub html_url: String,
    /// 克隆地址（SSH，可空）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_url: Option<String>,
    /// 克隆地址（HTTPS）
    pub clone_url: String,
    /// 是否为用户收藏的仓库（GitView 内部维护，与平台 star 无关）
    pub is_favorite: bool,
    /// 平台侧最近一次推送时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_pushed_at: Option<DateTime<Utc>>,
    /// GitView 同步该仓库元数据的时间
    pub synced_at: DateTime<Utc>,
}

/// 本地仓库元数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalRepository {
    /// 本地数据库主键
    pub id: String,
    /// 与远程仓库的关联（可空：用户手动添加未匹配到远端的本地仓库）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_repository_id: Option<String>,
    /// 仓库根目录绝对路径
    pub local_path: String,
    /// 当前分支（可空：detached HEAD 时为 None）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_branch: Option<String>,
    /// 远端 URL（取自 `git remote get-url origin`）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_url: Option<String>,
    /// 仓库状态摘要
    pub status: RepositoryStatus,
    /// 最近一次本地扫描刷新状态的时间
    pub last_checked_at: DateTime<Utc>,
    /// 记录创建时间
    pub created_at: DateTime<Utc>,
}

/// 扫描父目录的结果：新增的仓库列表 + 清理掉的失效记录数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    /// 本次扫描新增的本地仓库（不含已存在的）
    pub added: Vec<LocalRepository>,
    /// 本次清理掉的失效记录数（在扫描父目录之下、磁盘已不存在）
    pub removed: usize,
}
