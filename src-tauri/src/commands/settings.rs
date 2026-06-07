//! 设置相关 Tauri commands（US7 / T101）。
//!
//! 薄包装 `settings_service` 与 `git_cli_service` 的检测能力:
//!   - `get_settings` / `update_settings` — 读取 / 原子写入完整设置快照
//!   - `detect_git`                       — 自动探测 git（优先用设置中保存的路径）
//!   - `set_git_path`                     — 校验用户指定的 git 路径并持久化
//!
//! 设置不含凭据明文（token 走 keyring,见 credential_service）,故无需脱敏。

#![allow(clippy::needless_pass_by_value)]

use std::path::PathBuf;

use tauri::State;

use crate::errors::Result;
use crate::models::settings::{GitDetectionResult, Settings};
use crate::services::git_cli_service::{GitCliService, GitVersionInfo};
use crate::services::log_maintenance::{self, ClearLogsResult, LogStats};
use crate::services::settings_service;
use crate::AppState;

/// 读取完整设置快照（聚合四组,缺失项回退默认值）。
#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<Settings> {
    settings_service::get_settings(&state.db)
}

/// 原子写入完整设置快照。
#[tauri::command]
pub fn update_settings(state: State<'_, AppState>, settings: Settings) -> Result<()> {
    settings_service::update_settings(&state.db, &settings)
}

/// 探测可用的 git:优先用设置中保存的自定义路径,失败回退自动探测。
///
/// 「未检测到 git」是可处理的正常状态而非异常,故失败时返回 `found = false`,
/// 由前端引导用户安装或手动指定路径,而不是抛红打断流程。
#[tauri::command]
pub async fn detect_git(state: State<'_, AppState>) -> Result<GitDetectionResult> {
    // 先同步读出 preferred 路径,再进入 await,避免跨 await 持有数据库连接
    let preferred = settings_service::get_git(&state.db)?
        .git_executable_path
        .map(PathBuf::from);
    // 探测失败 = 未检测到 git（可处理的正常态）,回退 found=false 而非抛错
    let result = GitCliService::detect_with_preferred(preferred)
        .await
        .map_or_else(
            |_| GitDetectionResult {
                found: false,
                path: None,
                version: None,
                user_name: None,
                user_email: None,
            },
            |info| detection_from_info(&info, true),
        );
    Ok(result)
}

/// 校验用户指定的 git 路径,通过后持久化到 git 设置组。
///
/// 校验失败（路径无效 / 跑不了 `git --version`）直接返回错误,前端提示重选;
/// 只有校验通过才落库,避免把一个用不了的路径写进设置。
#[tauri::command]
pub async fn set_git_path(state: State<'_, AppState>, path: String) -> Result<GitDetectionResult> {
    let info = GitCliService::set_git_path(PathBuf::from(&path)).await?;
    // 校验通过才落库:仅更新可执行路径,保留 git 组其余字段（user.name 等）
    let mut git = settings_service::get_git(&state.db)?;
    git.git_executable_path = Some(info.path.to_string_lossy().into_owned());
    settings_service::set_git(&state.db, &git)?;
    Ok(detection_from_info(&info, true))
}

/// 把内部 `GitVersionInfo` 转成前端契约 `GitDetectionResult`。
///
/// 路径 / 版本一定有值（探测成功才调用）,user.name/email 透传 Option。
fn detection_from_info(info: &GitVersionInfo, found: bool) -> GitDetectionResult {
    GitDetectionResult {
        found,
        path: Some(info.path.to_string_lossy().into_owned()),
        version: Some(info.version.clone()),
        user_name: info.user_name.clone(),
        user_email: info.user_email.clone(),
    }
}

/// 读取日志目录占用统计（路径 + 大小 + 文件数），供设置页展示。
///
/// 统计无失败路径（目录缺失视为空），故直接返回 `LogStats` 而非 `Result`。
#[tauri::command]
#[must_use]
pub fn get_log_stats() -> LogStats {
    log_maintenance::log_stats()
}

/// 清理历史日志（保留当天、删除更早的滚动文件），返回删除数与释放字节。
///
/// 单文件删除失败被静默跳过、整体不报错，故直接返回 `ClearLogsResult`。
#[tauri::command]
#[must_use]
pub fn clear_old_logs() -> ClearLogsResult {
    log_maintenance::clear_old_logs()
}
