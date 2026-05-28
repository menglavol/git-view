//! 单仓库 Git 工作流相关 Tauri commands（US5 / T080）。
//!
//! 本模块是 US5「单仓库可视化 Git 工作流」用户故事的前端入口集合。
//! 共注册 **15 个** command，按职责分为五类：
//!   - **状态读取**：`git_status`、`git_diff`、`git_list_branches`、`git_log`
//!   - **暂存操作**：`git_stage_file`、`git_unstage_file`、
//!     `git_stage_all`、`git_unstage_all`
//!   - **提交**：`git_commit`
//!   - **网络操作**：`git_fetch`、`git_pull`、`git_push`
//!   - **分支管理**：`git_checkout_branch`、`git_create_branch`
//!   - **破坏性操作**：`git_discard_changes`（要求 `confirmed: true`，对应
//!     宪法 Principle III）
//!
//! 每个 command 接收 `repo_id`（对应 `local_repositories.id`），通过
//! `repository_service::load_local_repository` 解析仓库路径后转发到
//! `git_cli_service` 或 `git_reader_service`。

#![allow(clippy::needless_pass_by_value)]

use std::path::PathBuf;

use tauri::State;

use crate::errors::Result;
use crate::models::git::{Branch, CommitInfo, GitStatus};
use crate::services::git_cli_service::GitCliService;
use crate::services::git_reader_service::{self, DiffResult};
use crate::services::repository_service;
use crate::AppState;

/// 根据 repo_id 取出仓库的本地路径。
fn resolve_repo_path(state: &AppState, repo_id: &str) -> Result<PathBuf> {
    let repo = repository_service::load_local_repository(&state.db, repo_id)?;
    Ok(PathBuf::from(repo.local_path))
}

/// 构造默认 git CLI 服务（PATH 中的 git）。
fn make_git_cli() -> GitCliService {
    GitCliService::with_path(PathBuf::from("git"))
}

// =====================================================================
// 状态读取（4 个）
// =====================================================================

/// 读取工作区状态：当前分支、upstream、ahead/behind、文件变更列表。
#[tauri::command]
pub async fn git_status(state: State<'_, AppState>, repo_id: String) -> Result<GitStatus> {
    let path = resolve_repo_path(&state, &repo_id)?;
    git_reader_service::status(&path).await
}

/// 查看文件 diff（`cached = true` 查看暂存区相对 HEAD）。
///
/// `file = None` 时返回工作区所有变更的合并 diff；> 1MB 自动截断。
#[tauri::command]
pub async fn git_diff(
    state: State<'_, AppState>,
    repo_id: String,
    file: Option<String>,
    cached: Option<bool>,
) -> Result<DiffResult> {
    let path = resolve_repo_path(&state, &repo_id)?;
    git_reader_service::diff(&path, file.as_deref(), cached.unwrap_or(false)).await
}

/// 列出所有分支（含远端追踪分支）。
#[tauri::command]
pub async fn git_list_branches(state: State<'_, AppState>, repo_id: String) -> Result<Vec<Branch>> {
    let path = resolve_repo_path(&state, &repo_id)?;
    git_reader_service::list_branches(&path).await
}

/// 分页查询提交历史。
#[tauri::command]
pub async fn git_log(
    state: State<'_, AppState>,
    repo_id: String,
    page: Option<u32>,
    page_size: Option<u32>,
) -> Result<Vec<CommitInfo>> {
    let path = resolve_repo_path(&state, &repo_id)?;
    git_reader_service::log(&path, page.unwrap_or(0), page_size.unwrap_or(50)).await
}

// =====================================================================
// 暂存区操作（4 个）
// =====================================================================

/// 把单个文件加入暂存区。
#[tauri::command]
pub async fn git_stage_file(
    state: State<'_, AppState>,
    repo_id: String,
    file: String,
) -> Result<()> {
    let path = resolve_repo_path(&state, &repo_id)?;
    make_git_cli().stage_file(&path, &file).await
}

/// 把单个文件从暂存区移除（保留工作区修改）。
#[tauri::command]
pub async fn git_unstage_file(
    state: State<'_, AppState>,
    repo_id: String,
    file: String,
) -> Result<()> {
    let path = resolve_repo_path(&state, &repo_id)?;
    make_git_cli().unstage_file(&path, &file).await
}

/// 把当前工作区所有变更加入暂存区。
#[tauri::command]
pub async fn git_stage_all(state: State<'_, AppState>, repo_id: String) -> Result<()> {
    let path = resolve_repo_path(&state, &repo_id)?;
    make_git_cli().stage_all(&path).await
}

/// 清空整个暂存区（保留工作区修改）。
#[tauri::command]
pub async fn git_unstage_all(state: State<'_, AppState>, repo_id: String) -> Result<()> {
    let path = resolve_repo_path(&state, &repo_id)?;
    make_git_cli().unstage_all(&path).await
}

// =====================================================================
// 提交（1 个）
// =====================================================================

/// 提交已暂存的变更。
///
/// 调用流程：先执行 5 项前置校验（T081），通过后写入临时文件并
/// 执行 `git commit -F`。返回 stdout 摘要供前端展示。
#[tauri::command]
pub async fn git_commit(
    state: State<'_, AppState>,
    repo_id: String,
    message: String,
    description: Option<String>,
) -> Result<String> {
    let path = resolve_repo_path(&state, &repo_id)?;
    let cli = make_git_cli();
    cli.pre_commit_check(&path).await?;
    cli.commit(&path, &message, description.as_deref()).await
}

// =====================================================================
// 网络操作（3 个）
// =====================================================================

/// `git fetch --all --prune`。
#[tauri::command]
pub async fn git_fetch(state: State<'_, AppState>, repo_id: String) -> Result<String> {
    let path = resolve_repo_path(&state, &repo_id)?;
    make_git_cli().fetch(&path).await
}

/// `git pull --ff-only`，遇分叉或冲突返回中文友好错误。
#[tauri::command]
pub async fn git_pull(state: State<'_, AppState>, repo_id: String) -> Result<String> {
    let path = resolve_repo_path(&state, &repo_id)?;
    make_git_cli().pull(&path).await
}

/// `git push`，遇拒绝/无 upstream/鉴权失败返回中文友好错误。
#[tauri::command]
pub async fn git_push(state: State<'_, AppState>, repo_id: String) -> Result<String> {
    let path = resolve_repo_path(&state, &repo_id)?;
    make_git_cli().push(&path).await
}

// =====================================================================
// 分支管理（2 个）
// =====================================================================

/// 切换到指定分支。
///
/// 工作区不干净时返回 `DirtyWorkdir`（前端按错误码 disable 按钮）。
#[tauri::command]
pub async fn git_checkout_branch(
    state: State<'_, AppState>,
    repo_id: String,
    branch: String,
) -> Result<()> {
    let path = resolve_repo_path(&state, &repo_id)?;
    make_git_cli().checkout_branch(&path, &branch).await
}

/// 创建新分支，可选择是否立即切换。
#[tauri::command]
pub async fn git_create_branch(
    state: State<'_, AppState>,
    repo_id: String,
    name: String,
    checkout: Option<bool>,
) -> Result<()> {
    let path = resolve_repo_path(&state, &repo_id)?;
    make_git_cli()
        .create_branch(&path, &name, checkout.unwrap_or(false))
        .await
}

// =====================================================================
// 破坏性操作（1 个，Principle III）
// =====================================================================

/// 丢弃工作区变更。
///
/// **必须**通过前端 `ConfirmDangerDialog` 二次确认后才允许调用，并显式传入
/// `confirmed: true`。服务层会在 `confirmed = false` 时立即返回
/// `UserCancelled`，作为双重防御。
#[tauri::command]
pub async fn git_discard_changes(
    state: State<'_, AppState>,
    repo_id: String,
    files: Vec<String>,
    confirmed: bool,
) -> Result<()> {
    let path = resolve_repo_path(&state, &repo_id)?;
    let refs: Vec<&str> = files.iter().map(String::as_str).collect();
    make_git_cli()
        .discard_changes(&path, &refs, confirmed)
        .await
}
