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
//!
//! **日志策略**：只读与暂存类命令不写 operation_log（高频、无副作用），
//! 仅 commit / 网络 / 分支 / 破坏性这类有副作用的写操作才经
//! `log_git_result` 落库审计——详见各区块上方说明。

#![allow(clippy::needless_pass_by_value)]

use std::path::{Path, PathBuf};
use std::time::Instant;

use tauri::State;

use crate::db::pool::DbPool;
use crate::errors::{GitViewError, Result};
use crate::models::git::{Branch, CommitInfo, GitStatus};
use crate::models::operation_log::{OperationStatus, OperationType};
use crate::services::git_cli_service::GitCliService;
use crate::services::git_reader_service::{self, DiffResult};
use crate::services::{log_service, repository_service};
use crate::AppState;

/// 根据 repo_id 取出仓库的本地路径。
///
/// 所有 command 都以 `repo_id`（local_repositories 主键）为入参而非裸路径：
/// 避免前端持有并到处传递绝对路径，也让仓库被移除后的调用能立刻拿到
/// 明确的「仓库不存在」错误，而不是在后续 git 调用里报模糊的路径错误。
fn resolve_repo_path(state: &AppState, repo_id: &str) -> Result<PathBuf> {
    let repo = repository_service::load_local_repository(&state.db, repo_id)?;
    Ok(PathBuf::from(repo.local_path))
}

/// 构造默认 git CLI 服务（PATH 中的 git）。
///
/// GitView 不内置 git，直接复用系统 PATH 中的可执行文件；其可用性
/// 由 US7 的环境检测负责，这里只做最简单的默认构造，避免重复探测开销。
fn make_git_cli() -> GitCliService {
    GitCliService::with_path(PathBuf::from("git"))
}

/// 取仓库路径末段目录名作为日志 target（比 UUID 形式的 repo_id 更可读）。
///
/// 日志面向人阅读，目录名（如 `myrepo`）远比主键易懂；取不到末段时
/// 退化为完整路径，保证 target 永远非空。
fn repo_name(path: &Path) -> String {
    path.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

/// 统一记录一次 Git 操作日志（US6）。
///
/// 写入失败不影响主流程（log_service 内部已记 warn）。错误分类：
/// `UserCancelled` → Cancelled，其余错误 → Failed，无错误 → Success。
fn log_git_result(
    db: &DbPool,
    op: OperationType,
    target: &str,
    command: &str,
    start: Instant,
    error: Option<&GitViewError>,
    output: Option<&str>,
) {
    // 毫秒级耗时；操作耗时远不会溢出 u64，截断分支不可达
    #[allow(clippy::cast_possible_truncation)]
    let duration_ms = start.elapsed().as_millis() as u64;
    // 把领域错误映射成日志状态：用户主动取消不算失败，避免污染失败率统计
    let (status, err_msg) = match error {
        None => (OperationStatus::Success, None),
        Some(GitViewError::UserCancelled) => (OperationStatus::Cancelled, None),
        Some(e) => (OperationStatus::Failed, Some(e.to_string())),
    };
    // 忽略返回值：日志写入是「尽力而为」，绝不能因记日志失败而中断 Git 操作
    let _ = log_service::record_operation(
        db,
        op,
        target,
        status,
        Some(command),
        output,
        err_msg.as_deref(),
        duration_ms,
    );
}

// =====================================================================
// 状态读取（4 个）
//
// 这 4 个命令均为只读查询，且被前端高频调用（切换文件、轮询刷新）。
// 它们刻意不调用 `log_git_result`：只读操作没有副作用，若写入
// operation_log 只会用海量无意义记录淹没真正需要审计的写操作。
// =====================================================================

/// 读取工作区状态：当前分支、upstream、ahead/behind、文件变更列表。
///
/// 这是工作区视图的核心数据源，前端在每次文件操作后都会重新拉取，
/// 因此走轻量的 reader 层而非 CLI，尽量降低单次开销。
#[tauri::command]
pub async fn git_status(state: State<'_, AppState>, repo_id: String) -> Result<GitStatus> {
    let path = resolve_repo_path(&state, &repo_id)?;
    git_reader_service::status(&path).await
}

/// 查看文件 diff（`cached = true` 查看暂存区相对 HEAD）。
///
/// `file = None` 时返回工作区所有变更的合并 diff；> 1MB 自动截断。
/// `cached` 默认 false：前端默认展示「工作区相对暂存区」的未暂存改动，
/// 这是用户在工作区视图里最常关注的内容。
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
///
/// 含远端追踪分支：前端的分支切换下拉需要同时展示本地分支与 origin 分支，
/// 这样用户无需先 checkout 就能看到远端有哪些分支可切。
#[tauri::command]
pub async fn git_list_branches(state: State<'_, AppState>, repo_id: String) -> Result<Vec<Branch>> {
    let path = resolve_repo_path(&state, &repo_id)?;
    git_reader_service::list_branches(&path).await
}

/// 分页查询提交历史。
///
/// 默认从第 0 页、每页 50 条：与前端提交历史列表的首屏分页保持一致，
/// 避免一次性加载超大仓库的全部历史拖慢渲染。
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
//
// 暂存 / 取消暂存是纯本地索引操作，前端在勾选文件时会频繁触发。
// 与状态读取同理：失败会即时反馈给用户，无需进 operation_log 审计，
// 故这 4 个命令也不调用 `log_git_result`。
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
///
/// 仅退出索引、不碰工作区：对应 `git restore --staged`，用户的实际改动不丢。
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
///
/// 对应「全选暂存」入口，等价于 `git add -A`。
#[tauri::command]
pub async fn git_stage_all(state: State<'_, AppState>, repo_id: String) -> Result<()> {
    let path = resolve_repo_path(&state, &repo_id)?;
    make_git_cli().stage_all(&path).await
}

/// 清空整个暂存区（保留工作区修改）。
///
/// 对应「全部取消暂存」入口；同样不触碰工作区文件，只重置索引。
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
    let target = repo_name(&path);
    let cli = make_git_cli();
    let start = Instant::now();
    // 前置校验 + 提交作为整体记录：校验未通过也算一次失败的 commit。
    // 用 async 块把两步包成单个 Result，这样日志里一次 Commit 记录就能
    // 同时覆盖「校验失败」与「提交失败」，前端不必区分两种失败来源。
    let result = async {
        cli.pre_commit_check(&path).await?;
        cli.commit(&path, &message, description.as_deref()).await
    }
    .await;
    log_git_result(
        &state.db,
        OperationType::Commit,
        &target,
        "git commit",
        start,
        result.as_ref().err(),
        result.as_ref().ok().map(String::as_str),
    );
    result
}

// =====================================================================
// 网络操作（3 个）
//
// fetch / pull / push 都有远端副作用，是日志审计的重点对象，故均记录耗时与结果。
// 它们返回 `Result<String>`，String 即 stdout 摘要，会作为 output 一并入库，
// 便于用户事后排查「这次到底拉取 / 推送了什么」。
// =====================================================================

/// `git fetch --all --prune`。
///
/// `--prune` 会清掉本地已不存在于远端的追踪分支，保持分支列表干净，
/// 避免下拉里堆积早已被删除的远端分支。
#[tauri::command]
pub async fn git_fetch(state: State<'_, AppState>, repo_id: String) -> Result<String> {
    let path = resolve_repo_path(&state, &repo_id)?;
    let target = repo_name(&path);
    let start = Instant::now();
    let result = make_git_cli().fetch(&path).await;
    log_git_result(
        &state.db,
        OperationType::Fetch,
        &target,
        "git fetch --all --prune",
        start,
        result.as_ref().err(),
        result.as_ref().ok().map(String::as_str),
    );
    result
}

/// `git pull --ff-only`，遇分叉或冲突返回中文友好错误。
///
/// 只允许快进合并：避免在 GUI 里悄悄产生 merge commit 这种用户难以察觉的副作用，
/// 分叉时明确报错让用户自己决定如何整合。
#[tauri::command]
pub async fn git_pull(state: State<'_, AppState>, repo_id: String) -> Result<String> {
    let path = resolve_repo_path(&state, &repo_id)?;
    let target = repo_name(&path);
    let start = Instant::now();
    let result = make_git_cli().pull(&path).await;
    log_git_result(
        &state.db,
        OperationType::Pull,
        &target,
        "git pull --ff-only",
        start,
        result.as_ref().err(),
        result.as_ref().ok().map(String::as_str),
    );
    result
}

/// `git push`，遇拒绝/无 upstream/鉴权失败返回中文友好错误。
///
/// 三类失败各映射成中文错误码，前端据此决定是提示「设置 upstream」还是「检查凭据」，
/// 而不是把原始英文 stderr 直接抛给用户。
#[tauri::command]
pub async fn git_push(state: State<'_, AppState>, repo_id: String) -> Result<String> {
    let path = resolve_repo_path(&state, &repo_id)?;
    let target = repo_name(&path);
    let start = Instant::now();
    let result = make_git_cli().push(&path).await;
    log_git_result(
        &state.db,
        OperationType::Push,
        &target,
        "git push",
        start,
        result.as_ref().err(),
        result.as_ref().ok().map(String::as_str),
    );
    result
}

// =====================================================================
// 分支管理（2 个）
//
// 切换 / 创建分支同样有副作用需审计，但返回 `Result<()>` 没有 stdout，
// 因此记录日志时 output 传 None，仅保留命令、耗时与可能的错误信息。
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
    let target = repo_name(&path);
    let start = Instant::now();
    let result = make_git_cli().checkout_branch(&path, &branch).await;
    log_git_result(
        &state.db,
        OperationType::Checkout,
        &target,
        &format!("git checkout {branch}"),
        start,
        result.as_ref().err(),
        None,
    );
    result
}

/// 创建新分支，可选择是否立即切换。
///
/// `checkout` 默认 false：与命令行 `git branch <name>` 的「只建不切」语义一致，
/// 需要立即切换时由前端显式传 true（等价 `git checkout -b`）。
#[tauri::command]
pub async fn git_create_branch(
    state: State<'_, AppState>,
    repo_id: String,
    name: String,
    checkout: Option<bool>,
) -> Result<()> {
    let path = resolve_repo_path(&state, &repo_id)?;
    let target = repo_name(&path);
    let start = Instant::now();
    let result = make_git_cli()
        .create_branch(&path, &name, checkout.unwrap_or(false))
        .await;
    log_git_result(
        &state.db,
        OperationType::CreateBranch,
        &target,
        &format!("git branch {name}"),
        start,
        result.as_ref().err(),
        None,
    );
    result
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
    let target = repo_name(&path);
    // 转成 &str 切片传给服务层：CLI 封装按引用消费文件列表，避免不必要的克隆
    let refs: Vec<&str> = files.iter().map(String::as_str).collect();
    let start = Instant::now();
    let result = make_git_cli().discard_changes(&path, &refs, confirmed).await;
    log_git_result(
        &state.db,
        OperationType::DiscardChanges,
        &target,
        "git discard (checkout/clean)",
        start,
        result.as_ref().err(),
        None,
    );
    result
}
