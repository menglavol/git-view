//! 克隆任务领域模型。
//!
//! 描述一次仓库克隆任务的生命周期：从入队 → 运行 → 完成/失败/取消。
//! 用于支撑前端 "克隆任务列表" 页面与并发任务调度（V1 单任务串行）。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 克隆任务状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloneTaskStatus {
    /// 已入队，等待调度
    Pending,
    /// 正在克隆中
    Running,
    /// 已成功完成
    Completed,
    /// 失败
    Failed,
    /// 用户取消
    Cancelled,
}

/// 克隆任务实体。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloneTask {
    /// 任务唯一标识（UUID v4）
    pub id: String,
    /// 关联的远程仓库 ID
    pub remote_repository_id: String,
    /// 仓库展示名（冗余字段，便于前端无需 join 即可展示）
    pub repository_name: String,
    /// 克隆远端 URL（HTTPS 或 SSH，决定使用的协议）
    pub remote_url: String,
    /// 目标本地路径
    pub target_path: String,
    /// 要克隆的分支（None = 克隆远端默认分支，对应 `git clone` 不带 `--branch`）。
    /// 旧任务（迁移前入库）该列为 NULL，语义即「默认分支」，向后兼容。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// 任务状态
    pub status: CloneTaskStatus,
    /// 进度百分比（0-100，仅 `Running` 状态有意义）
    pub progress: u8,
    /// 失败时的错误信息（已脱敏）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    /// 入队时间
    pub created_at: DateTime<Utc>,
    /// 开始执行时间（可空）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    /// 完成时间（成功 / 失败 / 取消均填充）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<DateTime<Utc>>,
}
