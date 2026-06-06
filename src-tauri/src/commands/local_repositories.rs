//! 本地仓库相关 Tauri commands（US4 / T072）。
//!
//! 本模块是「本地仓库集中管理」用户故事的前端入口集合，仅做以下事项：
//!   - 接收前端 IPC 请求并解构参数
//!   - 调用 `services::repository_service` 已实现的业务函数
//!   - 对于「打开目录」「打开终端」两个跨平台桌面集成命令，
//!     在本文件内用 `std::process::Command` 直接 spawn 系统进程，
//!     避免扩散到 `tauri-plugin-shell` 的 capability 白名单
//!
//! 命令清单（9 个）：
//!   1. add_local_repository                    — 添加单个本地仓库
//!   2. scan_local_repositories                 — 扫描父目录批量添加
//!   3. list_local_repositories                 — 查询本地仓库列表
//!   4. remove_local_repository                 — 从列表移除（不删磁盘文件）
//!   5. refresh_local_repository_status         — 刷新单仓库 Git 状态
//!   6. refresh_all_local_repository_status     — 刷新所有仓库 Git 状态
//!   7. batch_fetch_repositories                — 并行 Fetch 多仓库
//!   8. open_repository_folder                  — 在系统文件管理器中打开仓库目录
//!   9. open_repository_in_terminal             — 在系统终端中打开仓库目录

#![allow(clippy::needless_pass_by_value)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use tauri::State;

use crate::errors::{GitViewError, Result};
use crate::models::operation_log::{OperationStatus, OperationType};
use crate::models::repository::{LocalRepository, ScanResult};
use crate::services::log_service;
use crate::services::repository_service::{self, BatchFetchSummary};
use crate::AppState;

// =====================================================================
// CRUD / 扫描 / 状态刷新（薄包装层）
// =====================================================================

/// 添加单个本地仓库。
///
/// 前端通常通过 `tauri-plugin-dialog` 让用户选择目录后传入路径；
/// 后端会校验目录存在性、是否为 Git 仓库、是否重复加入。
#[tauri::command]
pub async fn add_local_repository(
    state: State<'_, AppState>,
    path: String,
) -> Result<LocalRepository> {
    repository_service::add_local_repository(&state.db, Path::new(&path)).await
}

/// 扫描父目录下所有 Git 仓库并批量添加。
///
/// `max_depth` 控制 walkdir 递归层数，未传时默认 5（足以覆盖 `~/Projects/<group>/<repo>` 等场景）。
#[tauri::command]
pub async fn scan_local_repositories(
    state: State<'_, AppState>,
    root: String,
    max_depth: Option<usize>,
) -> Result<ScanResult> {
    // 未显式指定深度时默认下钻 5 层：足以覆盖 ~/Projects/<group>/<repo> 这类常见嵌套，
    // 又能避免在超深目录树上无谓地长时间扫描
    let depth = max_depth.unwrap_or(5);
    // 下面调用 service：新增本次扫到的仓库，
    // 并清理该父目录之下、磁盘已消失的旧记录（仅限该父目录子树，不动其它仓库）
    let start = Instant::now();
    let result =
        repository_service::scan_local_repositories(&state.db, Path::new(&root), depth).await;
    // 操作耗时远不会溢出 u64，截断分支不可达
    #[allow(clippy::cast_possible_truncation)]
    let duration_ms = start.elapsed().as_millis() as u64;
    // 扫描结果转为日志状态：成功记识别到的仓库数量，失败记脱敏后的错误原因
    let (status, output, err) = match &result {
        Ok(r) => (
            OperationStatus::Success,
            Some(format!(
                "新增 {} 个，移除 {} 个失效",
                r.added.len(),
                r.removed
            )),
            None,
        ),
        Err(e) => (OperationStatus::Failed, None, Some(e.to_string())),
    };
    // 扫描通常较慢，记一条带耗时的日志供性能回溯
    let _ = log_service::record_operation(
        &state.db,
        OperationType::ScanRepos,
        &root,
        status,
        Some("scan"),
        output.as_deref(),
        err.as_deref(),
        duration_ms,
    );
    result
}

/// 查询所有本地仓库。
#[tauri::command]
pub fn list_local_repositories(state: State<'_, AppState>) -> Result<Vec<LocalRepository>> {
    repository_service::list_local_repositories(&state.db)
}

/// 从列表移除本地仓库（仅删除数据库记录，不删除磁盘文件）。
///
/// 前端 MUST 在调用前完成二次确认（宪法 Principle III）。
#[tauri::command]
pub fn remove_local_repository(state: State<'_, AppState>, id: String) -> Result<()> {
    repository_service::remove_local_repository(&state.db, &id)
}

/// 刷新单个本地仓库的 Git 工作区状态。
#[tauri::command]
pub async fn refresh_local_repository_status(
    state: State<'_, AppState>,
    id: String,
) -> Result<LocalRepository> {
    repository_service::refresh_local_repository_status(&state.db, &id).await
}

/// 刷新所有本地仓库的 Git 工作区状态（顺序执行避免大量并发 git 子进程）。
#[tauri::command]
pub async fn refresh_all_local_repository_status(
    state: State<'_, AppState>,
) -> Result<Vec<LocalRepository>> {
    repository_service::refresh_all_local_repository_status(&state.db).await
}

/// 并行 Fetch 多个仓库（内部 Semaphore=4 控制并发），单仓库失败不阻塞其他。
#[tauri::command]
pub async fn batch_fetch_repositories(
    state: State<'_, AppState>,
    ids: Vec<String>,
) -> Result<BatchFetchSummary> {
    // 先记录请求数量：ids 会被 batch_fetch 消费（move），其后无法再取长度
    let count = ids.len();
    let start = Instant::now();
    // 单仓库失败不阻塞其他仓库，逐仓成败汇总在 BatchFetchSummary 中返回
    let result = repository_service::batch_fetch(&state.db, ids).await;
    #[allow(clippy::cast_possible_truncation)]
    let duration_ms = start.elapsed().as_millis() as u64;
    let (status, err) = match &result {
        Ok(_) => (OperationStatus::Success, None),
        Err(e) => (OperationStatus::Failed, Some(e.to_string())),
    };
    let _ = log_service::record_operation(
        &state.db,
        OperationType::Fetch,
        "批量 Fetch",
        status,
        Some("git fetch --all --prune"),
        Some(&format!("{count} 个仓库")),
        err.as_deref(),
        duration_ms,
    );
    result
}

// =====================================================================
// 桌面集成（打开目录 / 终端）
// ---------------------------------------------------------------------
// 这两个命令需要 spawn 系统进程；为避免给 `tauri-plugin-shell` 扩展
// `allow-execute` 权限（会放大攻击面），统一从 Rust 侧调用 `std::process::Command`。
// =====================================================================

/// 通过 id 查仓库后取 `local_path`，并校验目录仍然存在。
///
/// 与 `open_*` 两个命令共用，集中处理「记录已存在但磁盘目录被外部删除」的边界。
fn resolve_repo_path(state: &AppState, id: &str) -> Result<PathBuf> {
    let repo = repository_service::list_local_repositories(&state.db)?
        .into_iter()
        .find(|r| r.id == id)
        .ok_or_else(|| GitViewError::NotFound(format!("本地仓库 {id} 不存在")))?;

    let path = PathBuf::from(&repo.local_path);
    if !path.exists() {
        return Err(GitViewError::PathMissing(repo.local_path));
    }
    Ok(path)
}

/// 在系统文件管理器（Finder / 资源管理器 / 桌面文件管理器）中打开仓库目录。
///
/// 平台分支：
///   - macOS：`open <path>`（Finder）
///   - Windows：`explorer <path>`
///   - Linux：`xdg-open <path>`（依据 XDG 规范由用户桌面决定具体 file manager）
#[tauri::command]
pub fn open_repository_folder(state: State<'_, AppState>, id: String) -> Result<()> {
    let path = resolve_repo_path(&state, &id)?;
    let path_str = path.to_string_lossy().to_string();

    // 按编译目标选择命令；spawn 后立即返回，不等待外部进程结束
    let result = if cfg!(target_os = "macos") {
        Command::new("open").arg(&path_str).spawn()
    } else if cfg!(target_os = "windows") {
        Command::new("explorer").arg(&path_str).spawn()
    } else {
        // Linux / 其他 Unix：依赖 XDG MIME 关联（绝大多数桌面均支持）
        Command::new("xdg-open").arg(&path_str).spawn()
    };

    result
        .map(|_| ())
        .map_err(|e| GitViewError::GitCommand(format!("无法打开目录：{e}")))
}

/// 在系统终端中打开仓库目录。
///
/// 平台分支：
///   - macOS：`open -a Terminal <path>`（系统自带 Terminal.app；iTerm 用户可后续在设置中切换）
///   - Windows：优先 `wt -d <path>`（Windows Terminal），失败兜底 `cmd /c start cmd /K cd /d <path>`
///   - Linux：顺序尝试 `gnome-terminal` → `konsole` → `xterm`，任一可用即返回
#[tauri::command]
pub fn open_repository_in_terminal(state: State<'_, AppState>, id: String) -> Result<()> {
    let path = resolve_repo_path(&state, &id)?;
    let path_str = path.to_string_lossy().to_string();

    let spawn_result: std::io::Result<()> = if cfg!(target_os = "macos") {
        Command::new("open")
            .args(["-a", "Terminal", &path_str])
            .spawn()
            .map(|_| ())
    } else if cfg!(target_os = "windows") {
        // 优先 Windows Terminal；wt.exe 失败时兜底到经典 cmd
        match Command::new("wt").args(["-d", &path_str]).spawn() {
            Ok(_) => Ok(()),
            Err(_) => Command::new("cmd")
                .args(["/c", "start", "cmd", "/K", "cd", "/d", &path_str])
                .spawn()
                .map(|_| ()),
        }
    } else {
        // Linux 三档兜底：按主流桌面终端优先级顺序尝试
        try_linux_terminals(&path_str)
    };

    spawn_result.map_err(|e| GitViewError::GitCommand(format!("无法打开终端：{e}")))
}

/// Linux 平台终端 spawn 兜底实现：依次尝试 gnome-terminal、konsole、xterm。
///
/// 任一 spawn 成功即返回；全部失败时返回最后一次的 io::Error，由调用方
/// 统一转为 `GitViewError::GitCommand`。
#[cfg(target_os = "linux")]
fn try_linux_terminals(path: &str) -> std::io::Result<()> {
    // gnome-terminal --working-directory=<path>
    if Command::new("gnome-terminal")
        .arg(format!("--working-directory={path}"))
        .spawn()
        .is_ok()
    {
        return Ok(());
    }
    // konsole --workdir <path>
    if Command::new("konsole")
        .args(["--workdir", path])
        .spawn()
        .is_ok()
    {
        return Ok(());
    }
    // xterm 不支持工作目录参数，退而求其次：通过 -e 启动 shell 并 cd
    Command::new("xterm")
        .args(["-e", &format!("cd {path} && $SHELL")])
        .spawn()
        .map(|_| ())
}

/// 非 Linux 平台的占位实现（编译期不会被调用，仅满足类型签名一致性）。
#[cfg(not(target_os = "linux"))]
#[allow(dead_code)]
fn try_linux_terminals(_path: &str) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Linux terminal fallback called on non-Linux platform",
    ))
}
