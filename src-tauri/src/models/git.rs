//! Git 操作相关领域模型。
//!
//! 包括分支、提交、文件变更、工作区状态等 V1 范围内需要在 UI 展示的 Git 概念。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 单文件变更状态（对应 `git status --porcelain` 输出）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileStatus {
    /// 新增（未跟踪）
    Untracked,
    /// 已添加到暂存区
    Added,
    /// 已修改未暂存
    Modified,
    /// 已修改并暂存
    Staged,
    /// 已删除
    Deleted,
    /// 已重命名
    Renamed,
    /// 冲突
    Conflicted,
    /// 已忽略
    Ignored,
}

/// 单个文件变更条目。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    /// 文件相对仓库根的路径
    pub path: String,
    /// 重命名前的路径（仅 `Renamed` 状态有值）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_path: Option<String>,
    /// 变更状态
    pub status: FileStatus,
    /// 是否已加入暂存区（与 `status` 字段配合使用，区分工作区与索引状态）
    pub staged: bool,
}

/// Git 分支（含本地与远端追踪信息）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Branch {
    /// 分支名（不含 `refs/heads/` 前缀）
    pub name: String,
    /// 是否为当前 HEAD 指向的分支
    pub is_current: bool,
    /// 是否为远端分支（如 `origin/main`）
    pub is_remote: bool,
    /// 关联的 upstream 分支（可空）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream: Option<String>,
    /// 本地领先 upstream 的提交数（仅本地分支有值）
    pub ahead: u32,
    /// 本地落后 upstream 的提交数（仅本地分支有值）
    pub behind: u32,
    /// 最近一次提交的短哈希
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_commit_short: Option<String>,
}

/// 提交简要信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitInfo {
    /// 提交完整哈希
    pub sha: String,
    /// 提交短哈希（7 位）
    pub short_sha: String,
    /// 提交标题（首行）
    pub summary: String,
    /// 完整提交信息（含正文）
    pub message: String,
    /// 作者姓名
    pub author_name: String,
    /// 作者邮箱
    pub author_email: String,
    /// 提交时间
    pub authored_at: DateTime<Utc>,
    /// 父提交哈希列表（合并提交有多个父）
    pub parent_shas: Vec<String>,
}

/// 工作区聚合状态（一次性返回给前端的概览数据）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStatus {
    /// 当前分支（detached HEAD 时为 None）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_branch: Option<String>,
    /// 关联的 upstream
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream: Option<String>,
    /// 领先 upstream 的提交数
    pub ahead: u32,
    /// 落后 upstream 的提交数
    pub behind: u32,
    /// 所有文件变更条目
    pub changes: Vec<FileChange>,
    /// 工作区是否干净（无任何未提交变更）
    pub is_clean: bool,
}
