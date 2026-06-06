//! Clone 任务编排服务。
//!
//! 负责：
//!   - 任务持久化（CRUD 到 `clone_tasks` 表）
//!   - 并发上限控制（`tokio::sync::Semaphore`）
//!   - 任务生命周期：pending → running → completed/failed/cancelled
//!   - 进度事件 emit 到 Tauri 前端
//!   - 成功后自动加入 `local_repositories`（T060）
//!   - 目录组织策略计算（T058）

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use tauri::Emitter;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::db::pool::DbPool;
use crate::errors::{GitViewError, Result};
use crate::models::account::GitPlatform;
use crate::models::clone_task::{CloneTask, CloneTaskStatus};
use crate::models::operation_log::{OperationStatus, OperationType};
use crate::models::repository::RemoteRepository;
use crate::models::settings::DirectoryStrategy;
use crate::services::credential_service;
use crate::services::git_cli_service::{CloneProgressEvent, CredentialInjection, GitCliService};
use crate::services::log_service;
use crate::services::proxy::{git_proxy_env, resolve_proxy};
use crate::services::repository_service;
use crate::services::settings_service;
use crate::utils::path::ensure_dir_exists;
use crate::utils::redact::redact_token;

// =====================================================================
// 目录策略（T058）
// =====================================================================

/// 根据策略计算单个仓库的目标本地路径。
///
/// - `Flat`：`<root>/<name>`
/// - `ByOwner`：`<root>/<owner>/<name>`
/// - `ByPlatformAndOwner`：`<root>/<platform>/<owner>/<name>`
#[must_use]
pub fn compute_target_path(
    root: &Path,
    repo: &RemoteRepository,
    strategy: DirectoryStrategy,
) -> PathBuf {
    match strategy {
        DirectoryStrategy::Flat => root.join(&repo.name),
        DirectoryStrategy::ByOwner => root.join(&repo.owner).join(&repo.name),
        DirectoryStrategy::ByPlatformAndOwner => root
            .join(platform_dir(repo.platform))
            .join(&repo.owner)
            .join(&repo.name),
    }
}

const fn platform_dir(p: GitPlatform) -> &'static str {
    match p {
        GitPlatform::Github => "github",
        GitPlatform::Gitlab => "gitlab",
        GitPlatform::Gitee => "gitee",
    }
}

// =====================================================================
// 任务运行时管理（T057）
// =====================================================================

/// 任务调度运行时状态。
///
/// `Clone` + `Arc` 内部容器，便于在多个 Tauri command 调用间共享。
#[derive(Clone)]
pub struct CloneManager {
    /// 全局并发信号量（容量 = 用户配置的 concurrency，默认 3，上限 8）
    semaphore: Arc<Semaphore>,
    /// 正在运行任务的 cancel token 与远端 URL（key = task_id）
    handles: Arc<Mutex<HashMap<String, TaskHandle>>>,
    /// Git CLI 服务（持有 git 路径）
    git: Arc<GitCliService>,
}

struct TaskHandle {
    cancel: CancellationToken,
}

impl CloneManager {
    /// 创建管理器。`concurrency` 取值范围 [1, 8]。
    #[must_use]
    pub fn new(git: GitCliService, concurrency: u8) -> Self {
        let bounded = concurrency.clamp(1, 8) as usize;
        Self {
            semaphore: Arc::new(Semaphore::new(bounded)),
            handles: Arc::new(Mutex::new(HashMap::new())),
            git: Arc::new(git),
        }
    }

    /// 占位构造（无 git，仅用于 detect 失败时启动应用）。
    #[must_use]
    pub fn placeholder() -> Self {
        Self::new(GitCliService::with_path(PathBuf::from("git")), 3)
    }
}

// =====================================================================
// 入队 / 启动（T057）
// =====================================================================

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCloneTasksPayload {
    pub remote_repository_ids: Vec<String>,
    pub target_root: String,
    pub directory_strategy: DirectoryStrategy,
    pub concurrency: Option<u8>,
    pub auto_add_to_local: bool,
}

/// 批量创建 clone 任务（status = pending）。
///
/// 每个仓库 ID 对应一行 clone_tasks 记录；同时根据策略计算 target_path。
pub fn create_clone_tasks(
    pool: &DbPool,
    payload: &CreateCloneTasksPayload,
) -> Result<Vec<CloneTask>> {
    let now_iso = Utc::now().to_rfc3339();
    let target_root = PathBuf::from(&payload.target_root);
    let mut created = Vec::new();

    pool.with_conn(|conn| {
        conn.execute_batch("BEGIN;")?;

        for repo_id in &payload.remote_repository_ids {
            let repo: RemoteRepository =
                match repository_service::get_remote_repository_by_id(conn, repo_id) {
                    Ok(r) => r,
                    Err(e) => {
                        let _ = conn.execute_batch("ROLLBACK;");
                        return Err(e);
                    }
                };

            let target = compute_target_path(&target_root, &repo, payload.directory_strategy);
            let task_id = Uuid::new_v4().to_string();

            let insert = conn.execute(
                "INSERT INTO clone_tasks (
                    id, remote_repository_id, repository_name, remote_url,
                    target_path, status, progress, error_message,
                    created_at, started_at, finished_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, 'pending', 0, NULL, ?6, NULL, NULL)",
                params![
                    task_id,
                    repo.id,
                    repo.full_name,
                    repo.clone_url,
                    target.to_string_lossy(),
                    now_iso,
                ],
            );

            if let Err(e) = insert {
                let _ = conn.execute_batch("ROLLBACK;");
                return Err(GitViewError::from(e));
            }

            created.push(CloneTask {
                id: task_id,
                remote_repository_id: repo.id,
                repository_name: repo.full_name,
                remote_url: repo.clone_url,
                target_path: target.to_string_lossy().to_string(),
                status: CloneTaskStatus::Pending,
                progress: 0,
                error_message: None,
                created_at: Utc::now(),
                started_at: None,
                finished_at: None,
            });
        }

        conn.execute_batch("COMMIT;")?;
        Ok(())
    })?;

    Ok(created)
}

/// 启动指定的 pending 任务（spawn tokio task 执行）。
///
/// - `app_handle` 用于 emit Tauri 事件
/// - 每个任务进入 semaphore → 状态 running → 调用 clone → 完成事件
///
/// 参数按值传递的理由：内部循环对每个任务克隆一份后分别 move 到 spawn 闭包；
/// 接收 `&` 引用会强制调用方持有所有权而该函数实际消费了这些值的克隆。
#[allow(clippy::needless_pass_by_value)]
pub fn start_clone_tasks<R: tauri::Runtime>(
    app_handle: tauri::AppHandle<R>,
    pool: DbPool,
    manager: CloneManager,
    task_ids: Vec<String>,
    auto_add_to_local: bool,
) {
    for task_id in task_ids {
        let app = app_handle.clone();
        let pool = pool.clone();
        let mgr = manager.clone();
        tauri::async_runtime::spawn(async move {
            run_one_task(app, pool, mgr, task_id, auto_add_to_local).await;
        });
    }
}

// 单任务执行体集中保留 100+ 行：含状态机切换、进度回调、cancellation、
// 凭据清理与最终入库；拆分子函数会增加跨段共享 mutex/handle 的复杂度，
// 显式豁免 too_many_lines lint。
#[allow(clippy::too_many_lines)]
async fn run_one_task<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    pool: DbPool,
    manager: CloneManager,
    task_id: String,
    auto_add_to_local: bool,
) {
    // 任务记录不存在直接退出（已被外部清理）
    let Ok(task) = load_task(&pool, &task_id) else {
        return;
    };

    if !matches!(
        task.status,
        CloneTaskStatus::Pending | CloneTaskStatus::Failed
    ) {
        return;
    }

    // semaphore 关闭意味着应用正在 shutdown，安静退出
    let Ok(permit) = manager.semaphore.clone().acquire_owned().await else {
        return;
    };

    let cancel = CancellationToken::new();
    {
        // mutex 中毒（其他任务 panic 残留）时本任务跳过，不影响新任务
        let Ok(mut guard) = manager.handles.lock() else {
            return;
        };
        guard.insert(
            task_id.clone(),
            TaskHandle {
                cancel: cancel.clone(),
            },
        );
    }

    let _ = update_status(&pool, &task_id, CloneTaskStatus::Running, Some(0), None);
    emit_status(&app, &task_id, CloneTaskStatus::Running, 0, None);

    let target_path = PathBuf::from(&task.target_path);

    if target_path.exists() {
        let msg = format!("目标目录已存在：{}", target_path.display());
        let _ = update_status(&pool, &task_id, CloneTaskStatus::Failed, None, Some(&msg));
        emit_status(&app, &task_id, CloneTaskStatus::Failed, 0, Some(&msg));
        cleanup_handle(&manager, &task_id);
        drop(permit);
        return;
    }

    if let Some(parent) = target_path.parent() {
        if let Err(e) = ensure_dir_exists(parent) {
            let msg = format!("创建父目录失败：{e}");
            let _ = update_status(&pool, &task_id, CloneTaskStatus::Failed, None, Some(&msg));
            emit_status(&app, &task_id, CloneTaskStatus::Failed, 0, Some(&msg));
            cleanup_handle(&manager, &task_id);
            drop(permit);
            return;
        }
    }

    let credentials = build_credentials_for_task(&pool, &task);

    let app_for_progress = app.clone();
    let task_id_for_progress = task_id.clone();
    let pool_for_progress = pool.clone();
    let progress_cb = move |ev: CloneProgressEvent| {
        let pct = ev.percent;
        let _ = update_progress(&pool_for_progress, &task_id_for_progress, pct);
        let _ = app_for_progress.emit(
            "clone-task-progress",
            CloneProgressPayload {
                task_id: task_id_for_progress.clone(),
                stage: ev.stage,
                percent: pct,
            },
        );
    };

    // 读全局网络设置,把代理转成 git 子进程环境变量。
    // V1 clone 流程不区分账号级代理,统一用全局兜底;读设置失败回退默认（无代理）,
    // 不让设置异常阻断 clone。Explicit 才注入 HTTP(S)_PROXY,System/None 为空。
    let proxy_env = {
        let net = settings_service::get_network(&pool).unwrap_or_default();
        git_proxy_env(&resolve_proxy(&net, None, false))
    };

    let start = Instant::now();
    let clone_result = manager
        .git
        .clone_repository(
            &task.remote_url,
            &target_path,
            credentials,
            &proxy_env,
            progress_cb,
            cancel.clone(),
        )
        .await;

    // US6：记录 clone 操作日志（耗时为 clone 调用的近似时长）
    #[allow(clippy::cast_possible_truncation)]
    let duration_ms = start.elapsed().as_millis() as u64;

    match clone_result {
        Ok(()) => {
            let _ = update_status(&pool, &task_id, CloneTaskStatus::Completed, Some(100), None);
            emit_status(&app, &task_id, CloneTaskStatus::Completed, 100, None);
            let _ = log_service::record_operation(
                &pool,
                OperationType::Clone,
                &task.repository_name,
                OperationStatus::Success,
                Some("git clone"),
                None,
                None,
                duration_ms,
            );

            if auto_add_to_local {
                if let Err(e) = add_to_local_repositories(&pool, &task) {
                    tracing::warn!("自动加入本地仓库失败：{e}");
                }
            }
        }
        Err(GitViewError::UserCancelled) => {
            let _ = update_status(&pool, &task_id, CloneTaskStatus::Cancelled, None, None);
            emit_status(&app, &task_id, CloneTaskStatus::Cancelled, 0, None);
            let _ = log_service::record_operation(
                &pool,
                OperationType::Clone,
                &task.repository_name,
                OperationStatus::Cancelled,
                Some("git clone"),
                None,
                None,
                duration_ms,
            );
        }
        Err(e) => {
            let safe_msg = redact_token(&e.to_string());
            let _ = update_status(
                &pool,
                &task_id,
                CloneTaskStatus::Failed,
                None,
                Some(&safe_msg),
            );
            emit_status(&app, &task_id, CloneTaskStatus::Failed, 0, Some(&safe_msg));
            let _ = log_service::record_operation(
                &pool,
                OperationType::Clone,
                &task.repository_name,
                OperationStatus::Failed,
                Some("git clone"),
                None,
                Some(&safe_msg),
                duration_ms,
            );
        }
    }

    cleanup_handle(&manager, &task_id);
    drop(permit);
}

fn cleanup_handle(manager: &CloneManager, task_id: &str) {
    if let Ok(mut guard) = manager.handles.lock() {
        guard.remove(task_id);
    }
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct CloneProgressPayload {
    task_id: String,
    stage: String,
    percent: u8,
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct CloneStatusPayload {
    task_id: String,
    status: CloneTaskStatus,
    progress: u8,
    error_message: Option<String>,
}

fn emit_status<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    task_id: &str,
    status: CloneTaskStatus,
    progress: u8,
    error_message: Option<&str>,
) {
    let _ = app.emit(
        "clone-task-status-changed",
        CloneStatusPayload {
            task_id: task_id.to_string(),
            status,
            progress,
            error_message: error_message.map(String::from),
        },
    );
}

// =====================================================================
// 取消 / 重试 / 清理
// =====================================================================

pub fn cancel_clone_task(manager: &CloneManager, task_id: &str) -> Result<()> {
    let guard = manager
        .handles
        .lock()
        .map_err(|e| GitViewError::Internal(format!("锁损坏：{e}")))?;
    // if let / else 写法比等价的 map_or_else 更清晰（含副作用 cancel()）
    #[allow(clippy::option_if_let_else)]
    if let Some(handle) = guard.get(task_id) {
        handle.cancel.cancel();
        Ok(())
    } else {
        Err(GitViewError::NotFound(format!("任务 {task_id} 未在运行")))
    }
}

pub fn retry_clone_task<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    pool: &DbPool,
    manager: CloneManager,
    task_id: &str,
    auto_add_to_local: bool,
) -> Result<()> {
    let task = load_task(pool, task_id)?;
    if !matches!(
        task.status,
        CloneTaskStatus::Failed | CloneTaskStatus::Cancelled
    ) {
        return Err(GitViewError::Internal(
            "仅失败 / 已取消的任务可重试".to_string(),
        ));
    }
    update_status(pool, task_id, CloneTaskStatus::Pending, Some(0), None)?;
    start_clone_tasks(
        app,
        pool.clone(),
        manager,
        vec![task_id.to_string()],
        auto_add_to_local,
    );
    Ok(())
}

pub fn clear_finished_clone_tasks(pool: &DbPool) -> Result<usize> {
    pool.with_conn(|conn| {
        let n = conn
            .execute(
                "DELETE FROM clone_tasks
                 WHERE status IN ('completed', 'cancelled', 'failed')",
                [],
            )
            .map_err(GitViewError::from)?;
        Ok(n)
    })
}

// =====================================================================
// 启动期回扫（T117）
//   1. 把上次会话遗留的 running/pending 任务标记为 failed（消除「幽灵任务」）
//   2. 清理崩溃残留的 askpass 临时脚本（正常由 RAII drop 删除）
// =====================================================================

pub fn reconcile_orphan_tasks(pool: &DbPool) -> Result<usize> {
    let reconciled = pool.with_conn(|conn| {
        let n = conn
            .execute(
                "UPDATE clone_tasks
                 SET status = 'failed',
                     error_message = COALESCE(error_message, '应用上次未正常退出，任务被中断'),
                     finished_at = ?1
                 WHERE status IN ('running', 'pending')",
                params![Utc::now().to_rfc3339()],
            )
            .map_err(GitViewError::from)?;
        Ok(n)
    })?;

    // 顺带清理崩溃残留的 askpass 脚本：应用被强杀时 AskpassGuard 的 drop 不执行，
    // 含一次性凭据的脚本会滞留临时目录，启动时静默清掉（属本应用 housekeeping）。
    let cleaned =
        crate::services::git_cli_service::cleanup_orphan_askpass_scripts(&std::env::temp_dir());
    if cleaned > 0 {
        tracing::info!("清理 {cleaned} 个残留 askpass 脚本");
    }

    Ok(reconciled)
}

// =====================================================================
// 查询
// =====================================================================

pub fn list_clone_tasks(pool: &DbPool) -> Result<Vec<CloneTask>> {
    pool.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, remote_repository_id, repository_name, remote_url,
                    target_path, status, progress, error_message,
                    created_at, started_at, finished_at
             FROM clone_tasks
             ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], row_to_task)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    })
}

fn load_task(pool: &DbPool, task_id: &str) -> Result<CloneTask> {
    pool.with_conn(|conn| {
        conn.query_row(
            "SELECT id, remote_repository_id, repository_name, remote_url,
                    target_path, status, progress, error_message,
                    created_at, started_at, finished_at
             FROM clone_tasks WHERE id = ?1",
            [task_id],
            row_to_task,
        )
        .map_err(GitViewError::from)
    })
}

fn update_status(
    pool: &DbPool,
    task_id: &str,
    status: CloneTaskStatus,
    progress: Option<u8>,
    error_message: Option<&str>,
) -> Result<()> {
    let status_str = serialize_status(status);
    let now = Utc::now().to_rfc3339();
    let task_id = task_id.to_string();
    let err = error_message.map(String::from);

    pool.with_conn(move |conn| {
        match status {
            CloneTaskStatus::Running => {
                conn.execute(
                    "UPDATE clone_tasks
                     SET status = ?1, progress = COALESCE(?2, progress),
                         started_at = COALESCE(started_at, ?3)
                     WHERE id = ?4",
                    params![status_str, progress.map(i64::from), now, task_id],
                )?;
            }
            CloneTaskStatus::Completed | CloneTaskStatus::Failed | CloneTaskStatus::Cancelled => {
                conn.execute(
                    "UPDATE clone_tasks
                     SET status = ?1, progress = COALESCE(?2, progress),
                         error_message = ?3, finished_at = ?4
                     WHERE id = ?5",
                    params![status_str, progress.map(i64::from), err, now, task_id],
                )?;
            }
            CloneTaskStatus::Pending => {
                conn.execute(
                    "UPDATE clone_tasks
                     SET status = ?1, progress = COALESCE(?2, progress),
                         error_message = NULL, started_at = NULL, finished_at = NULL
                     WHERE id = ?3",
                    params![status_str, progress.map(i64::from), task_id],
                )?;
            }
        }
        Ok(())
    })
}

fn update_progress(pool: &DbPool, task_id: &str, percent: u8) -> Result<()> {
    let task_id = task_id.to_string();
    pool.with_conn(move |conn| {
        conn.execute(
            "UPDATE clone_tasks SET progress = ?1 WHERE id = ?2",
            params![i64::from(percent), task_id],
        )?;
        Ok(())
    })
}

fn build_credentials_for_task(pool: &DbPool, task: &CloneTask) -> Option<CredentialInjection> {
    if !task.remote_url.starts_with("http") {
        return None;
    }

    let account_id: Option<String> = pool
        .with_conn(|conn| {
            conn.query_row(
                "SELECT account_id FROM remote_repositories WHERE id = ?1",
                [&task.remote_repository_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(GitViewError::from)
        })
        .ok()
        .flatten();

    let account_id = account_id?;
    let token = credential_service::load_token(&account_id).ok()?;

    let username = pool
        .with_conn(|conn| {
            conn.query_row(
                "SELECT username FROM accounts WHERE id = ?1",
                [&account_id],
                |row| row.get::<_, String>(0),
            )
            .map_err(GitViewError::from)
        })
        .unwrap_or_else(|_| "git".to_string());

    Some(CredentialInjection { username, token })
}

// =====================================================================
// 自动加入本地仓库（T060）
// =====================================================================

fn add_to_local_repositories(pool: &DbPool, task: &CloneTask) -> Result<()> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let task = task.clone();
    pool.with_conn(move |conn| {
        let exists: Option<String> = conn
            .query_row(
                "SELECT id FROM local_repositories WHERE local_path = ?1",
                [&task.target_path],
                |row| row.get(0),
            )
            .optional()?;
        if exists.is_some() {
            return Ok(());
        }

        conn.execute(
            "INSERT INTO local_repositories (
                id, remote_repository_id, local_path, current_branch,
                remote_url, status, last_checked_at, created_at
            ) VALUES (?1, ?2, ?3, NULL, ?4, 'unknown', ?5, ?5)",
            params![
                id,
                task.remote_repository_id,
                task.target_path,
                task.remote_url,
                now,
            ],
        )?;
        Ok(())
    })
}

// =====================================================================
// 行映射 / 序列化
// =====================================================================

const fn serialize_status(s: CloneTaskStatus) -> &'static str {
    match s {
        CloneTaskStatus::Pending => "pending",
        CloneTaskStatus::Running => "running",
        CloneTaskStatus::Completed => "completed",
        CloneTaskStatus::Failed => "failed",
        CloneTaskStatus::Cancelled => "cancelled",
    }
}

fn deserialize_status(s: &str) -> CloneTaskStatus {
    match s {
        "running" => CloneTaskStatus::Running,
        "completed" => CloneTaskStatus::Completed,
        "failed" => CloneTaskStatus::Failed,
        "cancelled" => CloneTaskStatus::Cancelled,
        _ => CloneTaskStatus::Pending,
    }
}

fn row_to_task(row: &rusqlite::Row<'_>) -> rusqlite::Result<CloneTask> {
    use chrono::DateTime;

    let status_str: String = row.get("status")?;
    let created_at_str: String = row.get("created_at")?;
    let started_at_str: Option<String> = row.get("started_at")?;
    let finished_at_str: Option<String> = row.get("finished_at")?;

    let parse_dt = |s: &str| -> rusqlite::Result<chrono::DateTime<Utc>> {
        DateTime::parse_from_rfc3339(s)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        e.to_string(),
                    )),
                )
            })
    };

    Ok(CloneTask {
        id: row.get("id")?,
        remote_repository_id: row.get("remote_repository_id")?,
        repository_name: row.get("repository_name")?,
        remote_url: row.get("remote_url")?,
        target_path: row.get("target_path")?,
        status: deserialize_status(&status_str),
        // clamp(0, 100) 已保证 i64 ∈ [0,100] 范围，转 u8 不会丢符号；
        // clippy::cast_sign_loss 是过度保守，此处显式豁免。
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        progress: row.get::<_, i64>("progress")?.clamp(0, 100) as u8,
        error_message: row.get("error_message")?,
        created_at: parse_dt(&created_at_str)?,
        started_at: started_at_str.and_then(|s| parse_dt(&s).ok()),
        finished_at: finished_at_str.and_then(|s| parse_dt(&s).ok()),
    })
}

// =====================================================================
// 单元测试
// =====================================================================
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::models::repository::Visibility;

    fn make_repo(name: &str, owner: &str, platform: GitPlatform) -> RemoteRepository {
        RemoteRepository {
            id: "rid".to_string(),
            account_id: "aid".to_string(),
            platform,
            remote_id: "1".to_string(),
            full_name: format!("{owner}/{name}"),
            name: name.to_string(),
            owner: owner.to_string(),
            description: None,
            visibility: Visibility::Public,
            default_branch: "main".to_string(),
            html_url: format!("https://example.com/{owner}/{name}"),
            ssh_url: None,
            clone_url: format!("https://example.com/{owner}/{name}.git"),
            is_favorite: false,
            last_pushed_at: None,
            synced_at: Utc::now(),
        }
    }

    #[test]
    fn target_path_flat() {
        let repo = make_repo("web-app", "alice", GitPlatform::Github);
        let p = compute_target_path(Path::new("/proj"), &repo, DirectoryStrategy::Flat);
        assert_eq!(p, PathBuf::from("/proj/web-app"));
    }

    #[test]
    fn target_path_by_owner() {
        let repo = make_repo("web-app", "alice", GitPlatform::Github);
        let p = compute_target_path(Path::new("/proj"), &repo, DirectoryStrategy::ByOwner);
        assert_eq!(p, PathBuf::from("/proj/alice/web-app"));
    }

    #[test]
    fn target_path_by_platform_and_owner() {
        let repo = make_repo("web-app", "alice", GitPlatform::Gitlab);
        let p = compute_target_path(
            Path::new("/proj"),
            &repo,
            DirectoryStrategy::ByPlatformAndOwner,
        );
        assert_eq!(p, PathBuf::from("/proj/gitlab/alice/web-app"));
    }

    #[test]
    fn unicode_path_handling() {
        let repo = make_repo("中文项目", "用户", GitPlatform::Github);
        let p = compute_target_path(
            Path::new("/路径"),
            &repo,
            DirectoryStrategy::ByPlatformAndOwner,
        );
        assert_eq!(p, PathBuf::from("/路径/github/用户/中文项目"));
    }

    #[test]
    fn same_name_different_owners_dont_collide() {
        let r1 = make_repo("tools", "alice", GitPlatform::Github);
        let r2 = make_repo("tools", "bob", GitPlatform::Github);
        let p1 = compute_target_path(Path::new("/p"), &r1, DirectoryStrategy::ByOwner);
        let p2 = compute_target_path(Path::new("/p"), &r2, DirectoryStrategy::ByOwner);
        assert_ne!(p1, p2);
    }

    #[test]
    fn manager_clamps_concurrency() {
        let git = GitCliService::with_path(PathBuf::from("git"));
        let m = CloneManager::new(git, 99);
        assert_eq!(m.semaphore.available_permits(), 8);
    }

    #[test]
    fn manager_minimum_concurrency() {
        let git = GitCliService::with_path(PathBuf::from("git"));
        let m = CloneManager::new(git, 0);
        assert_eq!(m.semaphore.available_permits(), 1);
    }
}
