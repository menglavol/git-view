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

use crate::db::pool::DbPool;
use crate::errors::{GitViewError, Result};
use crate::models::operation_log::{OperationStatus, OperationType};
use crate::models::repository::{
    CreateRepoRequest, LocalRepository, RemoteRepository, ScanResult, Visibility,
};
use crate::services::git_cli_service::{CredentialInjection, GitCliService};
use crate::services::proxy::{git_proxy_env, resolve_proxy};
use crate::services::repository_service::{self, BatchFetchSummary};
use crate::services::{
    account_service, credential_service, git_reader_service, log_service, settings_service,
};
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

// =====================================================================
// 发布到远程（创建远程仓库 + 关联 + push）
// =====================================================================

/// 把一个尚无 origin 的本地仓库发布到远程平台。
///
/// 适用场景：本地仓库在 git 层没有任何 origin（用户最初点 Push 会报
/// "No configured push destination"）；本命令一站式完成「建仓 + 关联 + 推送」。
///
/// 执行顺序（分段处理「部分失败」，远程一旦建成绝不回滚删除）：
///   1. 校验本地仓库存在、当前无 origin、当前分支可推送；
///   2. 按 account_id 构造对应平台 provider；
///   3. 调平台 API 创建**空**仓库（auto_init 关闭，避免 push non-fast-forward）；
///   4. 立即把远程仓库落库，使其即便后续失败也在 GitView 中可见；
///   5. 按协议选 HTTPS / SSH 远端地址；
///   6. `git remote add origin <url>`；
///   7. 建立本地 ↔ 远程关联并回填 remote_url（origin 已配成事实，提前于 push）；
///   8. 准备凭据（HTTPS 注入账户 token，SSH 依赖本机 key）；
///   9. `git push -u origin <branch>` 推送并设置 upstream。
///
/// 任一步失败都返回带「已完成到哪一步」的中文提示，便于用户按提示手动补救。
#[tauri::command]
pub async fn publish_local_repository(
    state: State<'_, AppState>,
    repo_id: String,
    account_id: String,
    name: String,
    description: Option<String>,
    visibility: Visibility,
    protocol: String,
) -> Result<RemoteRepository> {
    let local = repository_service::load_local_repository(&state.db, &repo_id)?;
    let path = PathBuf::from(&local.local_path);
    if !path.exists() {
        return Err(GitViewError::PathMissing(local.local_path));
    }
    // 双重防御：已配置 origin 的仓库不应发布（前端按钮已按 remoteUrl 隐藏入口）
    if local.remote_url.as_deref().is_some_and(|u| !u.is_empty()) {
        return Err(GitViewError::PathConflict(
            "该本地仓库已配置远程 origin，无需重复发布".to_string(),
        ));
    }
    // 取当前分支：detached HEAD（或尚无任何提交）时无法推送
    let git_status = git_reader_service::status(&path).await?;
    let branch = git_status.current_branch.ok_or_else(|| {
        GitViewError::Internal(
            "当前处于 detached HEAD 或尚无提交，请先创建分支并提交后再发布".to_string(),
        )
    })?;

    let account = account_service::load_account_by_id(&state.db, &account_id)?;
    let provider = account_service::provider_for_account(&state.db, &account_id)?;

    // 3) 调平台 API 创建空仓库
    // auto_init 已在各 provider 内关闭，避免远程带初始 commit 导致后续 push 被拒
    let req = CreateRepoRequest {
        name,
        description,
        visibility,
    };
    let remote_repo = provider.create_repository(&req, &account_id).await?;

    // 立即落库：远程已建成功，之后任何步骤失败都保留它在 GitView 可见
    repository_service::insert_remote_repository(&state.db, &remote_repo)?;

    // 按协议选择远端地址
    let is_ssh = protocol.eq_ignore_ascii_case("ssh");
    let remote_url = if is_ssh {
        remote_repo.ssh_url.clone().ok_or_else(|| {
            GitViewError::Internal("该平台未提供 SSH 克隆地址，请改用 HTTPS 协议发布".to_string())
        })?
    } else {
        remote_repo.clone_url.clone()
    };

    // 默认 git：沿用系统 PATH 中的可执行文件，与其他 git 命令保持一致
    let cli = GitCliService::with_path(PathBuf::from("git"));
    // 读全局网络设置并转成 git 代理环境变量（与 clone 一致）；读失败回退无代理。
    // 这是关键：reqwest 走代理建仓成功后，git push 也必须走同一代理，否则直连远端超时。
    let proxy_env = {
        let net = settings_service::get_network(&state.db).unwrap_or_default();
        git_proxy_env(&resolve_proxy(&net, None, false))
    };
    // start 计时覆盖 remote add + push 整体过程，供操作日志记录耗时
    let start = Instant::now();

    // 6) git remote add origin <url>：把新建仓库登记为本地 origin
    if let Err(e) = cli.add_remote(&path, "origin", &remote_url).await {
        // 远程已建、本地关联失败：保留远程并提示手动 add remote 的补救方式
        log_publish(
            &state.db,
            &remote_repo.full_name,
            &remote_repo.html_url,
            start,
            Some(&e),
        );
        return Err(GitViewError::GitCommand(format!(
            "远程仓库已创建成功（{}），但本地关联 origin 失败：{e}。可手动执行 git remote add origin {remote_url}",
            remote_repo.html_url
        )));
    }

    // 7) origin 已配成事实，立即建立本地 ↔ 远程关联（回填 remote_url + remote_repository_id）。
    // 提前到 push 之前：即便后续 push 失败，本地也已正确标记为「已关联」，状态保持一致。
    repository_service::link_local_to_remote(&state.db, &repo_id, &remote_repo.id, &remote_url)?;

    // 8) 准备凭据：HTTPS 用账户 token（经一次性 askpass 注入），SSH 依赖本机 key
    let credentials = if is_ssh {
        // SSH 不注入凭据，依赖本机已配置的 key
        None
    } else {
        // HTTPS：从 keyring 取 token，用户名取账户 username
        let token = credential_service::load_token(&account_id)?;
        Some(CredentialInjection {
            username: account.username.clone(),
            token,
        })
    };

    // 9) git push -u origin <branch>：推送当前分支并设置 upstream
    let push_result = cli
        .push_set_upstream(&path, "origin", &branch, credentials, &proxy_env)
        .await;
    // 无论成败都记一条发布日志（含整体耗时）
    log_publish(
        &state.db,
        &remote_repo.full_name,
        &remote_repo.html_url,
        start,
        push_result.as_ref().err(),
    );
    // 远程已建并关联、仅推送失败：保留现状并提示稍后手动 push（不回滚）
    if let Err(e) = push_result {
        return Err(GitViewError::GitCommand(format!(
            "远程仓库已创建并关联成功（{}），但推送失败：{e}。可稍后在仓库内手动执行 git push -u origin {branch}",
            remote_repo.html_url
        )));
    }

    // 全流程成功，返回新建远程仓库供前端刷新与展示
    Ok(remote_repo)
}

/// 记录一次「发布到远程」操作日志。
///
/// 归入 `Push` 类型（V1 OperationType 无 Publish 变体），command 字段注明 publish 以便区分。
fn log_publish(
    db: &DbPool,
    target: &str,
    html_url: &str,
    start: Instant,
    error: Option<&GitViewError>,
) {
    // 操作耗时远不会溢出 u64，截断分支不可达
    #[allow(clippy::cast_possible_truncation)]
    let duration_ms = start.elapsed().as_millis() as u64;
    // 用户主动取消不会出现在发布路径，故只区分成功 / 失败两类
    let (status, err_msg) = error.map_or((OperationStatus::Success, None), |e| {
        (OperationStatus::Failed, Some(e.to_string()))
    });
    let _ = log_service::record_operation(
        db,
        OperationType::Push,
        target,
        status,
        Some("publish（create remote + push -u origin）"),
        Some(html_url),
        err_msg.as_deref(),
        duration_ms,
    );
}
