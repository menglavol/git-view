//! Git CLI 调用封装。
//!
//! 提供：
//!   - Git 可执行文件检测与版本读取（T053）
//!   - `clone` 命令异步执行 + 进度事件流（T054）
//!   - stderr 进度解析器（T055）
//!   - HTTPS 临时凭据注入（GIT_ASKPASS 脚本，T056）
//!
//! 安全约束（宪法 Principle III）：
//!   - 远端 URL 不携带 token（凭据通过 GIT_ASKPASS 临时脚本注入）
//!   - 临时脚本随机命名，任务结束立即删除（RAII drop guard）
//!   - clone 时强制 `-c credential.helper=` 屏蔽全局 helper

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;

use once_cell::sync::Lazy;
use regex::Regex;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::errors::{GitViewError, Result};
use crate::models::git::FileStatus;
use crate::utils::process::apply_no_window;
use crate::utils::redact::redact_token;

/// Git 检测结果。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitVersionInfo {
    /// Git 可执行文件绝对路径
    pub path: PathBuf,
    /// `git --version` 解析出的版本号
    pub version: String,
    /// 全局 user.name（可空）
    pub user_name: Option<String>,
    /// 全局 user.email（可空）
    pub user_email: Option<String>,
}

/// HTTPS 凭据注入信息。
///
/// 使用 `GIT_ASKPASS` 脚本机制，token 仅在临时脚本文件中存在，
/// **不会**出现在子进程命令行参数或 `.git/config` 中。
#[derive(Clone)]
pub struct CredentialInjection {
    /// 用户名（如 GitHub 的 `oauth2` 或账号 username）
    pub username: String,
    /// 凭据 token / password
    pub token: String,
}

/// Clone 进度事件。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloneProgressEvent {
    /// 阶段名称（snake_case）
    pub stage: String,
    /// 当前阶段百分比（0-100）
    pub percent: u8,
    /// 原始 stderr 行（已脱敏）
    pub raw_line: String,
}

// =====================================================================
// Git CLI 服务（T053）
// =====================================================================

/// Git CLI 调用服务。
///
/// 通过 `detect()` 创建实例（自动从 PATH / 常见安装位置探测），
/// 或 `with_path(path)` 指定自定义可执行文件路径。
pub struct GitCliService {
    git_path: PathBuf,
}

impl GitCliService {
    /// 探测系统 Git 可执行文件（自动从 PATH / 常见安装位置查找）。
    ///
    /// 返回包含路径、版本号、`user.name`/`user.email` 的检测结果。
    pub async fn detect() -> Result<GitVersionInfo> {
        let git_path = locate_git_executable().await?;
        probe_git(git_path).await
    }

    /// 优先用设置中保存的自定义路径检测,失败回退自动探测（T100）。
    ///
    /// 用户在设置里指定过 git 路径时优先采信;若该路径已失效（卸载 / 移动）,
    /// **不直接报错**,而是回退到 PATH / 常见位置自动探测——避免「设置过期」
    /// 把整个 Git 功能卡死,让用户至少还能用系统里其它可用的 git。
    pub async fn detect_with_preferred(preferred: Option<PathBuf>) -> Result<GitVersionInfo> {
        if let Some(path) = preferred {
            // 用户指定路径优先;探测失败（返回 Err）则静默落到自动探测
            if let Ok(info) = probe_git(path).await {
                return Ok(info);
            }
        }
        Self::detect().await
    }

    /// 校验用户指定的 Git 路径并返回其检测信息（T100）。
    ///
    /// 只校验、**不写库**;持久化由 command 层调 `settings_service::set_git` 负责。
    /// 路径不是有效文件时直接返回 `GitNotFound`,给前端比「命令执行失败」更准确的反馈;
    /// 文件存在但跑不了 `git --version` 同样视为无效路径。
    pub async fn set_git_path(path: PathBuf) -> Result<GitVersionInfo> {
        if !path.is_file() {
            return Err(GitViewError::GitNotFound);
        }
        probe_git(path).await
    }

    /// 使用指定的 Git 路径构造服务。
    #[must_use]
    pub const fn with_path(git_path: PathBuf) -> Self {
        Self { git_path }
    }

    /// 获取当前使用的 Git 路径。
    #[must_use]
    pub fn git_path(&self) -> &Path {
        &self.git_path
    }

    /// 执行 Git 子命令并返回完整输出。
    pub async fn run(
        &self,
        args: &[&str],
        cwd: Option<&Path>,
        extra_env: &[(&str, &str)],
    ) -> Result<std::process::Output> {
        let mut cmd = Command::new(&self.git_path);
        // Windows 上隐藏控制台窗口，避免每次 git 调用闪现终端（其它平台无操作）
        apply_no_window(&mut cmd);
        cmd.args(args)
            .env("LC_ALL", "C")
            .env("GIT_TERMINAL_PROMPT", "0");
        for (k, v) in extra_env {
            cmd.env(k, v);
        }
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }
        cmd.output()
            .await
            .map_err(|e| GitViewError::GitCommand(format!("git 子进程失败：{e}")))
    }

    /// 异步执行 git clone，并通过 progress 回调推送解析后的进度事件。
    ///
    /// `cancel_token` 触发后立即 kill 子进程并清理目标目录。
    /// 失败或取消时清理半成品：`preserve_target_dir = true`（目标是用户预先建好的
    /// 空目录）时只清空内容、保留目录本身；否则删除整个 target_path。
    ///
    /// 参数较多（凭据、代理、进度回调、取消令牌等正交关注点），强行打包结构体反而
    /// 模糊语义，显式豁免 too_many_arguments。
    #[allow(clippy::too_many_arguments)]
    pub async fn clone_repository<F>(
        &self,
        remote_url: &str,
        target_path: &Path,
        preserve_target_dir: bool,
        credentials: Option<CredentialInjection>,
        extra_env: &[(String, String)],
        progress: F,
        cancel_token: CancellationToken,
    ) -> Result<()>
    where
        F: Fn(CloneProgressEvent) + Send + Sync + 'static,
    {
        let askpass_guard = match &credentials {
            Some(cred) => Some(AskpassGuard::create(cred)?),
            None => None,
        };

        let mut cmd = Command::new(&self.git_path);
        // Windows 上隐藏控制台窗口，避免 clone 时闪现终端（其它平台无操作）
        apply_no_window(&mut cmd);
        cmd.arg("clone")
            .arg("--progress")
            .arg("-c")
            .arg("credential.helper=")
            .arg(remote_url)
            .arg(target_path)
            .env("LC_ALL", "C")
            .env("GIT_TERMINAL_PROMPT", "0")
            .stdout(Stdio::null())
            .stderr(Stdio::piped());

        // 注入调用方提供的额外环境变量（代理 HTTP_PROXY/HTTPS_PROXY,见 proxy::git_proxy_env）。
        // 放在固定 env 之后、askpass 之前:代理与凭据互不覆盖,各自独立的环境键。
        for (key, value) in extra_env {
            cmd.env(key, value);
        }

        if let Some(guard) = &askpass_guard {
            cmd.env("GIT_ASKPASS", guard.script_path());
            // 部分 Git 版本走 SSH_ASKPASS 路径（无 controlling terminal 时）
            cmd.env("SSH_ASKPASS", guard.script_path());
            cmd.env("DISPLAY", ":0");
            if let Some(user) = guard.username_env() {
                cmd.env("GIT_USERNAME", user);
            }
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| GitViewError::GitCommand(format!("启动 git clone 失败：{e}")))?;

        let progress_arc = Arc::new(progress);
        let parser_handle = spawn_progress_reader(&mut child, Arc::clone(&progress_arc));

        let exit_status = tokio::select! {
            res = child.wait() => res.map_err(|e| {
                GitViewError::GitCommand(format!("等待 git clone 退出失败：{e}"))
            })?,
            () = cancel_token.cancelled() => {
                let _ = child.kill().await;
                cleanup_partial_clone(target_path, preserve_target_dir);
                drop(askpass_guard);
                return Err(GitViewError::UserCancelled);
            }
        };

        // 取出 reader 累积的 stderr 末尾行，供失败分类（如 SSH 公钥缺失）
        let stderr_tail = parser_handle.await.unwrap_or_default();

        if !exit_status.success() {
            cleanup_partial_clone(target_path, preserve_target_dir);
            let code = exit_status.code().unwrap_or(-1);
            return Err(classify_clone_failure(code, &stderr_tail));
        }

        Ok(())
    }

    // =====================================================================
    // 工作区写入操作（T077 — US5 单仓库工作流）
    // =====================================================================

    /// 把单个文件加入暂存区。
    pub async fn stage_file(&self, repo: &Path, file: &str) -> Result<()> {
        let output = self.run(&["add", "--", file], Some(repo), &[]).await?;
        ensure_success(&output, "git add")
    }

    /// 把当前工作区全部变更加入暂存区（含未跟踪文件）。
    pub async fn stage_all(&self, repo: &Path) -> Result<()> {
        let output = self.run(&["add", "-A"], Some(repo), &[]).await?;
        ensure_success(&output, "git add -A")
    }

    /// 把单个文件从暂存区移除（保留工作区修改）。
    pub async fn unstage_file(&self, repo: &Path, file: &str) -> Result<()> {
        let output = self
            .run(&["restore", "--staged", "--", file], Some(repo), &[])
            .await?;
        ensure_success(&output, "git restore --staged")
    }

    /// 清空整个暂存区（保留工作区修改）。
    pub async fn unstage_all(&self, repo: &Path) -> Result<()> {
        let output = self
            .run(&["restore", "--staged", "."], Some(repo), &[])
            .await?;
        ensure_success(&output, "git restore --staged .")
    }

    /// 通过临时文件机制提交，规避命令行转义对多行/中文/特殊字符的破坏。
    ///
    /// 步骤：
    ///   1. 拼接 `message` 与可选 `description`（中间空一行，遵循 Git 习惯）
    ///   2. 写入 `.git/COMMIT_GITVIEW`（位于仓库 .git 目录，属于 Git 自身
    ///      housekeeping 范畴，Principle III 允许直接读写）
    ///   3. `git commit -F <file> --cleanup=strip`
    ///   4. 无论成败均删除临时文件
    ///
    /// 返回 stdout 文本（含新 commit 摘要），便于上层记录操作日志。
    pub async fn commit(
        &self,
        repo: &Path,
        message: &str,
        description: Option<&str>,
    ) -> Result<String> {
        if message.trim().is_empty() {
            return Err(GitViewError::Internal(
                "commit message 不能为空".to_string(),
            ));
        }

        let commit_file = repo.join(".git").join("COMMIT_GITVIEW");
        let mut body = message.trim().to_string();
        if let Some(desc) = description {
            let desc_trimmed = desc.trim();
            if !desc_trimmed.is_empty() {
                body.push_str("\n\n");
                body.push_str(desc_trimmed);
            }
        }
        body.push('\n');

        std::fs::write(&commit_file, &body)
            .map_err(|e| GitViewError::Internal(format!("写入 COMMIT_GITVIEW 失败：{e}")))?;

        let commit_file_str = commit_file.to_string_lossy().into_owned();
        let run_result = self
            .run(
                &["commit", "-F", &commit_file_str, "--cleanup=strip"],
                Some(repo),
                &[],
            )
            .await;

        // 无论 commit 是否成功，都尝试清理临时文件；忽略删除失败（next 次 commit 会覆盖）
        let _ = std::fs::remove_file(&commit_file);

        let output = run_result?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GitViewError::GitCommand(format!(
                "git commit 失败：{stderr}"
            )));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    /// 丢弃工作区变更（不可恢复）。
    ///
    /// **安全约束（宪法 Principle III）**：仅当 `confirmed = true` 时执行；
    /// 调用方必须先通过 `ConfirmDangerDialog` 让用户输入关键词确认，再传
    /// `confirmed: true`。否则返回 `UserCancelled` 作为双重防御。
    ///
    /// 实现策略：先 `git checkout HEAD -- <files>` 恢复已跟踪文件，再
    /// `git clean -fd -- <files>` 清理未跟踪文件。两步独立执行，只要其中
    /// 一步成功即视为成功（适配混合已跟踪/未跟踪的文件集合）；都失败时
    /// 汇总 stderr 返回。
    pub async fn discard_changes(
        &self,
        repo: &Path,
        files: &[&str],
        confirmed: bool,
    ) -> Result<()> {
        if !confirmed {
            return Err(GitViewError::UserCancelled);
        }
        if files.is_empty() {
            return Ok(());
        }

        let mut checkout_args: Vec<&str> = vec!["checkout", "HEAD", "--"];
        checkout_args.extend(files.iter().copied());
        let checkout_out = self.run(&checkout_args, Some(repo), &[]).await?;

        let mut clean_args: Vec<&str> = vec!["clean", "-fd", "--"];
        clean_args.extend(files.iter().copied());
        let clean_out = self.run(&clean_args, Some(repo), &[]).await?;

        if !checkout_out.status.success() && !clean_out.status.success() {
            let stderr_checkout = String::from_utf8_lossy(&checkout_out.stderr);
            let stderr_clean = String::from_utf8_lossy(&clean_out.stderr);
            return Err(GitViewError::GitCommand(format!(
                "git discard 失败：checkout={stderr_checkout} clean={stderr_clean}"
            )));
        }
        Ok(())
    }

    // =====================================================================
    // 网络相关操作（T078 — fetch / pull / push）
    // =====================================================================

    /// 同步所有远程信息并清理已删除的远端追踪分支。
    ///
    /// 对应 spec FR-040：`git fetch --all --prune`。
    /// 输出经 `redact_token` 脱敏后返回，便于上层写入操作日志。
    pub async fn fetch(&self, repo: &Path) -> Result<String> {
        let output = self
            .run(&["fetch", "--all", "--prune"], Some(repo), &[])
            .await?;
        let stderr = redact_token(&String::from_utf8_lossy(&output.stderr));
        if !output.status.success() {
            return Err(map_network_failure("fetch", &stderr));
        }
        let stdout = redact_token(&String::from_utf8_lossy(&output.stdout));
        Ok(format!("{stdout}{stderr}"))
    }

    /// 快进合并远程更新。
    ///
    /// 对应 spec FR-040 / FR-041：默认使用 `--ff-only`，遇 non-fast-forward
    /// 或冲突时返回带中文翻译的 `GitCommand` 错误，前端可按关键词区分提示。
    pub async fn pull(&self, repo: &Path) -> Result<String> {
        let output = self.run(&["pull", "--ff-only"], Some(repo), &[]).await?;
        let stderr = redact_token(&String::from_utf8_lossy(&output.stderr));
        if !output.status.success() {
            let lower = stderr.to_lowercase();
            if lower.contains("not possible to fast-forward")
                || lower.contains("non-fast-forward")
                || lower.contains("diverged")
            {
                return Err(GitViewError::GitCommand(format!(
                    "Pull 失败：本地与远程分支已分叉，无法快进合并，请使用外部工具解决冲突后再继续。{stderr}"
                )));
            }
            if lower.contains("conflict") || lower.contains("merge_msg") {
                return Err(GitViewError::GitCommand(format!(
                    "Pull 后存在冲突，请使用外部工具（如 VS Code 或 git mergetool）解决冲突后再提交。{stderr}"
                )));
            }
            return Err(map_network_failure("pull", &stderr));
        }
        let stdout = redact_token(&String::from_utf8_lossy(&output.stdout));
        Ok(format!("{stdout}{stderr}"))
    }

    /// 推送当前分支到远程。
    ///
    /// 对应 spec FR-042：解析常见拒绝原因（non-fast-forward / no upstream /
    /// permission denied）映射到带中文友好提示的错误。
    pub async fn push(&self, repo: &Path) -> Result<String> {
        let output = self.run(&["push"], Some(repo), &[]).await?;
        let stderr = redact_token(&String::from_utf8_lossy(&output.stderr));
        if !output.status.success() {
            let lower = stderr.to_lowercase();
            if lower.contains("non-fast-forward") || lower.contains("rejected") {
                return Err(GitViewError::GitCommand(format!(
                    "推送被拒绝（远程有新提交），请先 Pull 远程更新后再推送。{stderr}"
                )));
            }
            if lower.contains("no upstream") || lower.contains("set-upstream") {
                return Err(GitViewError::GitCommand(format!(
                    "当前分支没有 upstream，请配置上游分支后再推送（git push -u origin <branch>）。{stderr}"
                )));
            }
            if lower.contains("permission denied") || lower.contains("authentication failed") {
                return Err(GitViewError::Forbidden);
            }
            return Err(map_network_failure("push", &stderr));
        }
        let stdout = redact_token(&String::from_utf8_lossy(&output.stdout));
        Ok(format!("{stdout}{stderr}"))
    }

    // =====================================================================
    // 发布到远程（remote add + 带凭据 push）
    // =====================================================================

    /// 添加一个远程（`git remote add <name> <url>`）。
    ///
    /// 「发布到远程」首步：把平台新建的仓库地址登记为本地 origin。
    pub async fn add_remote(&self, repo: &Path, name: &str, url: &str) -> Result<()> {
        let output = self
            .run(&["remote", "add", name, url], Some(repo), &[])
            .await?;
        ensure_success(&output, "git remote add")
    }

    /// 修改已有远程的 URL（`git remote set-url <name> <url>`）。
    ///
    /// 用于本地仓库协议切换（HTTPS↔SSH）：只改写 `.git/config` 的 remote url，
    /// 不触碰工作区，因此无需干净工作区即可执行。
    pub async fn set_remote_url(&self, repo: &Path, name: &str, url: &str) -> Result<()> {
        let output = self
            .run(&["remote", "set-url", name, url], Some(repo), &[])
            .await?;
        ensure_success(&output, "git remote set-url")
    }

    /// 推送当前分支到全新远程并设置 upstream（`git push -u <remote> <branch>`）。
    ///
    /// 与 [`Self::push`] 的关键区别：本方法面向**刚创建的 origin**，必须注入凭据。
    /// 复用 clone 的 `AskpassGuard` 临时脚本机制——token 不进命令行、不落
    /// `.git/config`，并以 `-c credential.helper=` 屏蔽全局 helper 强制走一次性凭据。
    /// HTTPS 传 `credentials = Some(..)`；SSH 传 `None`（靠本机 key 认证）。
    ///
    /// `proxy_env` 与 clone 一致：注入 `HTTP_PROXY`/`HTTPS_PROXY` 让 git 经代理访问远端，
    /// 避免「reqwest 走代理建仓成功、git 直连远端超时」的不一致（见 services::proxy::git_proxy_env）。
    pub async fn push_set_upstream(
        &self,
        repo: &Path,
        remote: &str,
        branch: &str,
        credentials: Option<CredentialInjection>,
        proxy_env: &[(String, String)],
    ) -> Result<String> {
        let askpass_guard = match &credentials {
            Some(cred) => Some(AskpassGuard::create(cred)?),
            None => None,
        };
        // 脚本路径需在整个 run() 期间存活，提前取出为 owned String（不借用 guard）
        let script_path = askpass_guard
            .as_ref()
            .map(|g| g.script_path().to_string_lossy().into_owned());

        let mut extra_env: Vec<(&str, &str)> = Vec::new();
        // 先注入代理（与 clone 一致）：reqwest 走代理时 git 也必须走，否则直连远端超时
        for (key, value) in proxy_env {
            extra_env.push((key.as_str(), value.as_str()));
        }
        if let Some(sp) = &script_path {
            extra_env.push(("GIT_ASKPASS", sp.as_str()));
            // 无 controlling terminal 的环境下部分 Git 走 SSH_ASKPASS 路径
            extra_env.push(("SSH_ASKPASS", sp.as_str()));
            extra_env.push(("DISPLAY", ":0"));
        }

        let output = self
            .run(
                &["-c", "credential.helper=", "push", "-u", remote, branch],
                Some(repo),
                &extra_env,
            )
            .await?;
        // 推送结束后再释放 guard（删除 askpass 脚本）
        drop(askpass_guard);

        let stderr = redact_token(&String::from_utf8_lossy(&output.stderr));
        if !output.status.success() {
            let lower = stderr.to_lowercase();
            // 仓库保护规则拦截（GitHub Push Protection 检测到密钥、受保护分支等）。
            // 这类输出同样含 "rejected"，必须先于 non-fast-forward 判断，否则会误报"远程已有提交"
            if lower.contains("push protection")
                || lower.contains("repository rule violations")
                || lower.contains("cannot contain secrets")
                || lower.contains("secret scanning")
                || lower.contains("protected branch")
            {
                return Err(GitViewError::GitCommand(format!(
                    "推送被远程仓库的保护规则拦截（如检测到提交包含密钥）：请按平台提示放行，或先从提交中移除敏感信息后重试。{stderr}"
                )));
            }
            // 真正的 non-fast-forward：远程已有本地缺失的提交
            if lower.contains("non-fast-forward") || lower.contains("fetch first") {
                return Err(GitViewError::GitCommand(format!(
                    "推送被拒绝（远程已存在提交）：请确认远程为空仓库或先 Pull 后重试。{stderr}"
                )));
            }
            if lower.contains("permission denied") || lower.contains("authentication failed") {
                return Err(GitViewError::Forbidden);
            }
            return Err(map_network_failure("push", &stderr));
        }
        let stdout = redact_token(&String::from_utf8_lossy(&output.stdout));
        Ok(format!("{stdout}{stderr}"))
    }

    // =====================================================================
    // 分支管理操作（T079 — checkout / create_branch）
    // =====================================================================

    /// 切换到指定分支。
    ///
    /// 对应 spec FR-044：调用前必先校验工作区干净，存在未提交变更时返回
    /// `DirtyWorkdir`，由前端按错误码 disable 切换按钮并 tooltip 提示。
    pub async fn checkout_branch(&self, repo: &Path, name: &str) -> Result<()> {
        let status = crate::services::git_reader_service::status(repo).await?;
        if !status.is_clean {
            return Err(GitViewError::DirtyWorkdir);
        }
        let output = self.run(&["checkout", name], Some(repo), &[]).await?;
        ensure_success(&output, "git checkout")
    }

    /// 创建新分支，可选择是否立即切换。
    ///
    /// 新建分支本身不修改工作区文件，因此 **不受脏工作区阻断**；
    /// 但 `checkout = true` 时若 git 自身判定与未提交变更冲突，会由 git
    /// 本身返回错误并由 `ensure_success` 透传。
    pub async fn create_branch(&self, repo: &Path, name: &str, checkout: bool) -> Result<()> {
        let args: Vec<&str> = if checkout {
            vec!["checkout", "-b", name]
        } else {
            vec!["branch", name]
        };
        let output = self.run(&args, Some(repo), &[]).await?;
        ensure_success(
            &output,
            if checkout {
                "git checkout -b"
            } else {
                "git branch"
            },
        )
    }

    /// 从远程分支 checkout 出本地分支并自动设置 upstream。
    ///
    /// 等价于 `git checkout -b <local> <remote>`。同样受脏工作区阻断。
    pub async fn checkout_remote_branch(
        &self,
        repo: &Path,
        remote_branch: &str,
        local_name: &str,
    ) -> Result<()> {
        let status = crate::services::git_reader_service::status(repo).await?;
        if !status.is_clean {
            return Err(GitViewError::DirtyWorkdir);
        }
        let output = self
            .run(
                &["checkout", "-b", local_name, remote_branch],
                Some(repo),
                &[],
            )
            .await?;
        ensure_success(&output, "git checkout -b (remote)")
    }

    // =====================================================================
    // commit 前置校验（T081 — FR-038 五项校验）
    // =====================================================================

    /// commit 前置 5 项校验。
    ///
    /// 对应 spec FR-038 与 US5 Acceptance Scenario 6/7。任一未通过即返回
    /// 含中文原因的 `Internal` 错误，调用方可在前端直接展示给用户。
    ///
    /// 校验项：
    ///   1. Git `user.name` 已配置（全局或仓库级）
    ///   2. Git `user.email` 已配置
    ///   3. 非 detached HEAD
    ///   4. 非 conflict 状态（无 Conflicted 文件）
    ///   5. 已暂存文件 > 0
    ///
    /// 注：message 是否为空由 [`Self::commit`] 自身校验，本函数仅检查环境。
    pub async fn pre_commit_check(&self, repo: &Path) -> Result<()> {
        if !self.git_config_present(repo, "user.name").await {
            return Err(GitViewError::Internal(
                "Git user.name 未配置，请在设置中配置 Git 身份后再提交".to_string(),
            ));
        }
        if !self.git_config_present(repo, "user.email").await {
            return Err(GitViewError::Internal(
                "Git user.email 未配置，请在设置中配置 Git 身份后再提交".to_string(),
            ));
        }

        let status = crate::services::git_reader_service::status(repo).await?;

        if status.current_branch.is_none() {
            return Err(GitViewError::Internal(
                "当前处于 detached HEAD 状态，请先创建分支后再提交".to_string(),
            ));
        }

        let has_conflict = status
            .changes
            .iter()
            .any(|c| c.status == FileStatus::Conflicted);
        if has_conflict {
            return Err(GitViewError::Internal(
                "工作区存在冲突文件，请先解决冲突后再提交".to_string(),
            ));
        }

        let has_staged = status.changes.iter().any(|c| c.staged);
        if !has_staged {
            return Err(GitViewError::Internal(
                "没有已暂存的文件，请先 stage 要提交的文件".to_string(),
            ));
        }

        Ok(())
    }

    /// 检查指定 git config key 是否存在且非空（先查仓库级，回退全局）。
    async fn git_config_present(&self, repo: &Path, key: &str) -> bool {
        let Ok(output) = self.run(&["config", "--get", key], Some(repo), &[]).await else {
            return false;
        };
        output.status.success() && !output.stdout.is_empty()
    }
}

/// 把网络 Git 命令的 stderr 映射到合适的错误变体。
///
/// 优先识别明确的语义（鉴权失败 / DNS 失败），其余兜底为 `GitCommand`。
fn map_network_failure(label: &str, stderr: &str) -> GitViewError {
    let lower = stderr.to_lowercase();
    if lower.contains("could not resolve host") || lower.contains("name or service not known") {
        return GitViewError::Network(format!("DNS 解析失败：{stderr}"));
    }
    if lower.contains("authentication failed") || lower.contains("permission denied") {
        return GitViewError::Forbidden;
    }
    if lower.contains("timed out") || lower.contains("operation timed out") {
        return GitViewError::Network(format!("连接超时：{stderr}"));
    }
    GitViewError::GitCommand(format!("git {label} 失败：{stderr}"))
}

/// 根据 git clone 退出码与 stderr 末尾行分类失败原因。
///
/// 重点识别 SSH 公钥缺失/未授权（错误消息带「SSH 认证失败」标记，前端据此
/// 引导用户去平台配置 SSH key）；其余回退为含退出码与简要 stderr 的通用错误。
fn classify_clone_failure(code: i32, stderr_tail: &[String]) -> GitViewError {
    let joined = stderr_tail.join("\n");
    let lower = joined.to_lowercase();
    // publickey 出现在 "Permission denied (publickey)"；host key verification failed
    // 是首次连接未信任主机 —— 两者都属本机 SSH 未就绪，引导去平台配公钥
    if lower.contains("publickey") || lower.contains("host key verification failed") {
        return GitViewError::GitCommand(format!(
            "SSH 认证失败（缺少或未授权 SSH 密钥）：请在对应平台账户设置中添加本机 SSH 公钥后重试。{joined}"
        ));
    }
    if joined.is_empty() {
        GitViewError::GitCommand(format!("git clone 退出码 {code}"))
    } else {
        GitViewError::GitCommand(format!("git clone 退出码 {code}：{joined}"))
    }
}

/// 校验子进程退出状态，失败时把 stderr 拼入错误信息。
fn ensure_success(output: &std::process::Output, label: &str) -> Result<()> {
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(GitViewError::GitCommand(format!("{label} 失败：{stderr}")))
    }
}

// =====================================================================
// Git 探测辅助
// =====================================================================

/// 在 PATH 与常见安装位置查找 git 可执行文件。
async fn locate_git_executable() -> Result<PathBuf> {
    let candidates = candidate_paths();

    for path in &candidates {
        let mut probe = Command::new(path);
        // Windows 上隐藏控制台窗口，避免探测 git 时闪现终端（其它平台无操作）
        apply_no_window(&mut probe);
        // .is_ok_and 比 .map(...).unwrap_or(false) 更直白
        if probe
            .arg("--version")
            .env("LC_ALL", "C")
            .output()
            .await
            .is_ok_and(|o| o.status.success())
        {
            return Ok(path.clone());
        }
    }

    Err(GitViewError::GitNotFound)
}

/// 探测指定路径的 git:跑 `git --version` 解析版本 + 读全局 user.name/email。
///
/// 从 detect 提取的复用单元,供 detect / detect_with_preferred / set_git_path 共用,
/// 避免「校验可执行 + 解析版本 + 读身份」三处逻辑重复。
async fn probe_git(git_path: PathBuf) -> Result<GitVersionInfo> {
    let mut version_cmd = Command::new(&git_path);
    // Windows 上隐藏控制台窗口，避免读取 git 版本时闪现终端（其它平台无操作）
    apply_no_window(&mut version_cmd);
    let version_output = version_cmd
        .arg("--version")
        .env("LC_ALL", "C")
        .output()
        .await
        .map_err(|e| GitViewError::GitCommand(format!("git --version 失败：{e}")))?;

    if !version_output.status.success() {
        return Err(GitViewError::GitNotFound);
    }

    let version_text = String::from_utf8_lossy(&version_output.stdout);
    let trimmed = version_text.trim();
    // 去掉 "git version " 前缀;没有该前缀时回退原始 trim 文本
    let version = trimmed
        .strip_prefix("git version ")
        .unwrap_or(trimmed)
        .to_string();

    // 全局未配置身份很常见,读不到不算错,用 .ok() 转 Option
    let user_name = read_git_config(&git_path, "user.name").await.ok();
    let user_email = read_git_config(&git_path, "user.email").await.ok();

    Ok(GitVersionInfo {
        path: git_path,
        version,
        user_name,
        user_email,
    })
}

#[cfg(target_os = "macos")]
fn candidate_paths() -> Vec<PathBuf> {
    vec![
        PathBuf::from("git"),
        PathBuf::from("/usr/bin/git"),
        PathBuf::from("/opt/homebrew/bin/git"),
        PathBuf::from("/usr/local/bin/git"),
    ]
}

#[cfg(target_os = "linux")]
fn candidate_paths() -> Vec<PathBuf> {
    vec![
        PathBuf::from("git"),
        PathBuf::from("/usr/bin/git"),
        PathBuf::from("/usr/local/bin/git"),
    ]
}

#[cfg(target_os = "windows")]
fn candidate_paths() -> Vec<PathBuf> {
    vec![
        PathBuf::from("git.exe"),
        PathBuf::from(r"C:\Program Files\Git\bin\git.exe"),
        PathBuf::from(r"C:\Program Files\Git\cmd\git.exe"),
        PathBuf::from(r"C:\Program Files (x86)\Git\bin\git.exe"),
    ]
}

async fn read_git_config(git_path: &Path, key: &str) -> Result<String> {
    let mut cmd = Command::new(git_path);
    // Windows 上隐藏控制台窗口，避免读取 git 配置时闪现终端（其它平台无操作）
    apply_no_window(&mut cmd);
    let output = cmd
        .args(["config", "--global", "--get", key])
        .env("LC_ALL", "C")
        .output()
        .await
        .map_err(|e| GitViewError::GitCommand(e.to_string()))?;
    if !output.status.success() {
        return Err(GitViewError::NotFound(format!("未配置 {key}")));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

// =====================================================================
// Clone 进度解析器（T055）
// =====================================================================

// 正则在程序启动期一次性编译；expect 失败属于源码 bug，无运行时恢复路径。
#[allow(clippy::expect_used)]
static RE_ENUMERATING: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^Enumerating objects:\s*(\d+)").expect("Enumerating 正则编译失败"));

#[allow(clippy::expect_used)]
static RE_PCT: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\d+)%\s*\(").expect("百分比正则编译失败"));

#[allow(clippy::expect_used)]
fn parse_progress_line(line: &str) -> Option<CloneProgressEvent> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let safe_line = redact_token(trimmed);

    let (stage, percent) = if trimmed.starts_with("Cloning into") {
        ("initializing", 0_u8)
    } else if let Some(caps) = RE_ENUMERATING.captures(trimmed) {
        let _ = caps;
        ("enumerating", 0)
    } else if trimmed.starts_with("Counting objects:") {
        ("counting", parse_percent(trimmed))
    } else if trimmed.starts_with("Compressing objects:") {
        ("compressing", parse_percent(trimmed))
    } else if trimmed.starts_with("Receiving objects:") {
        ("receiving", parse_percent(trimmed))
    } else if trimmed.starts_with("Resolving deltas:") {
        ("resolving", parse_percent(trimmed))
    } else if trimmed.starts_with("Updating files:") {
        ("checkout", parse_percent(trimmed))
    } else {
        return None;
    };

    Some(CloneProgressEvent {
        stage: stage.to_string(),
        percent,
        raw_line: safe_line,
    })
}

fn parse_percent(line: &str) -> u8 {
    RE_PCT
        .captures(line)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<u8>().ok())
        .unwrap_or(0)
        .min(100)
}

/// 拆分包含 `\r` 与 `\n` 的混合输出行。
///
/// Git 进度输出在同一阶段内用 `\r` 覆盖行；切换阶段时输出 `\n`。
/// 我们把两种分隔符均视为"行结束"，独立解析。
fn split_progress_lines(buf: &str) -> impl Iterator<Item = &str> {
    // `|c| matches!(c, '\r' | '\n')` 不必要，多字符 split 用数组更直观
    buf.split(['\r', '\n'])
}

fn spawn_progress_reader<F>(
    child: &mut Child,
    progress: Arc<F>,
) -> tokio::task::JoinHandle<Vec<String>>
where
    F: Fn(CloneProgressEvent) + Send + Sync + 'static,
{
    // 拿不到 stderr（极少见，比如已被消费）时返回空行集合，等价于 noop
    let Some(stderr) = child.stderr.take() else {
        return tokio::spawn(async { Vec::new() });
    };

    tokio::spawn(async move {
        // 仅保留最近 MAX_TAIL 行 stderr（已脱敏），供 clone 失败时识别错误（如 SSH 公钥缺失）；
        // const 置于块首以满足 clippy items-after-statements。
        const MAX_TAIL: usize = 30;
        let mut reader = BufReader::new(stderr);
        let mut chunk = Vec::new();
        // 滚动窗口：超过上限丢最旧行，避免大仓库进度刷屏导致无限增长
        let mut tail: std::collections::VecDeque<String> = std::collections::VecDeque::new();
        loop {
            // Ok(0) 表示 EOF，Err 表示 IO 异常——两者均退出循环
            match reader.read_until(b'\n', &mut chunk).await {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
            let text = String::from_utf8_lossy(&chunk).to_string();
            chunk.clear();
            for line in split_progress_lines(&text) {
                let trimmed = line.trim();
                // 非空行累积进 tail（超出上限丢最旧的），进度行另行解析推送
                if !trimmed.is_empty() {
                    tail.push_back(redact_token(trimmed));
                    if tail.len() > MAX_TAIL {
                        tail.pop_front();
                    }
                }
                if let Some(ev) = parse_progress_line(line) {
                    (progress)(ev);
                }
            }
        }
        tail.into_iter().collect()
    })
}

/// 清理 clone 失败/取消后的残留目录。
///
/// `preserve_dir = true`（目标目录是用户预先建好的空目录）时，只删除目录内
/// clone 产生的内容、**保留目录本身**；否则（目录由本次 clone 创建）整体删除。
fn cleanup_partial_clone(target_path: &Path, preserve_dir: bool) {
    if !target_path.exists() {
        return;
    }
    if preserve_dir {
        // 只清空目录内容，保留用户预建的目录本身
        if let Ok(entries) = std::fs::read_dir(target_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let _ = std::fs::remove_dir_all(&path);
                } else {
                    let _ = std::fs::remove_file(&path);
                }
            }
        }
    } else {
        let _ = std::fs::remove_dir_all(target_path);
    }
}

// =====================================================================
// GIT_ASKPASS 临时脚本（T056）
// =====================================================================

/// askpass 临时脚本统一文件名前缀。
///
/// 创建（`AskpassGuard::create`）与崩溃残留清理
/// （`cleanup_orphan_askpass_scripts`，T117）共用此常量，
/// 避免两处命名漂移导致「清理匹配不到自己创建的文件」。
const ASKPASS_PREFIX: &str = "gitview-askpass-";

/// RAII 守护：构造时创建 askpass 脚本，drop 时删除。
pub struct AskpassGuard {
    script_path: PathBuf,
    username: Option<String>,
}

impl AskpassGuard {
    /// 在系统临时目录创建 askpass 脚本。
    ///
    /// 脚本内容根据 stdin 提示决定输出 username 或 token：
    ///   - 提示含 "Username" → 输出 username
    ///   - 否则 → 输出 token
    pub fn create(cred: &CredentialInjection) -> Result<Self> {
        let id = Uuid::new_v4();
        let dir = std::env::temp_dir();
        let script_path = if cfg!(windows) {
            dir.join(format!("{ASKPASS_PREFIX}{id}.bat"))
        } else {
            dir.join(format!("{ASKPASS_PREFIX}{id}.sh"))
        };

        let content = if cfg!(windows) {
            // Windows .bat: 简化为只输出 token
            // GIT_ASKPASS 通常对 HTTPS 仅询问 password
            format!(
                "@echo off\r\nif \"%~1\"==\"\" goto print\r\necho %1 | findstr /i \"Username\" >nul && (echo {user}) || (echo {tok})\r\ngoto :eof\r\n:print\r\necho {tok}\r\n",
                user = cred.username,
                tok = cred.token,
            )
        } else {
            format!(
                "#!/bin/sh\ncase \"$1\" in\n  *Username*) printf '%s' '{user}' ;;\n  *) printf '%s' '{tok}' ;;\nesac\n",
                user = shell_escape(&cred.username),
                tok = shell_escape(&cred.token),
            )
        };

        std::fs::write(&script_path, content)
            .map_err(|e| GitViewError::Internal(format!("写入 askpass 脚本失败：{e}")))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o700);
            std::fs::set_permissions(&script_path, perms)
                .map_err(|e| GitViewError::Internal(format!("设置 askpass 权限失败：{e}")))?;
        }

        Ok(Self {
            script_path,
            username: Some(cred.username.clone()),
        })
    }

    fn script_path(&self) -> &Path {
        &self.script_path
    }

    fn username_env(&self) -> Option<&str> {
        self.username.as_deref()
    }
}

impl Drop for AskpassGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.script_path);
    }
}

/// 清理系统临时目录下残留的 askpass 脚本，返回删除的文件数（T117）。
///
/// 正常路径由 [`AskpassGuard`] 的 `Drop` 删除脚本；但应用崩溃 / 被强杀时
/// `Drop` 不会执行，含一次性凭据的脚本会滞留在临时目录。启动期回扫调用本
/// 函数把这些残留清掉，属本应用 housekeeping，静默执行无需用户确认。
///
/// **安全约束**：只按本应用前缀 [`ASKPASS_PREFIX`] 匹配、不递归子目录，
/// 绝不误删用户或其它程序的临时文件；单个文件删除失败（如被占用）仅跳过，
/// 不中断整体清理。
#[must_use]
pub fn cleanup_orphan_askpass_scripts(dir: &Path) -> usize {
    // 临时目录读不到（不存在 / 无权限）时直接返回 0，回扫在任何环境都不应 panic。
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };

    let mut removed = 0;
    for entry in entries.flatten() {
        // 仅匹配本应用前缀，且删除成功才计数；二者用 && 短路避免误删后多计。
        if entry
            .file_name()
            .to_string_lossy()
            .starts_with(ASKPASS_PREFIX)
            && std::fs::remove_file(entry.path()).is_ok()
        {
            removed += 1;
        }
    }
    removed
}

/// 把字符串包装为 POSIX 单引号转义形式，防止脚本注入。
fn shell_escape(s: &str) -> String {
    // 单引号内只需替换 ' 为 '\''
    s.replace('\'', "'\\''")
}

// =====================================================================
// 单元测试
// =====================================================================
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn parse_initializing() {
        let ev = parse_progress_line("Cloning into 'foo'...").unwrap();
        assert_eq!(ev.stage, "initializing");
        assert_eq!(ev.percent, 0);
    }

    #[test]
    fn parse_counting_with_percent() {
        let ev = parse_progress_line("Counting objects: 35% (7/20), done.").unwrap();
        assert_eq!(ev.stage, "counting");
        assert_eq!(ev.percent, 35);
    }

    #[test]
    fn parse_receiving_high_percent() {
        let ev =
            parse_progress_line("Receiving objects: 99% (495/500), 1.20 MiB | 800 KiB/s").unwrap();
        assert_eq!(ev.stage, "receiving");
        assert_eq!(ev.percent, 99);
    }

    #[test]
    fn parse_resolving_deltas() {
        let ev = parse_progress_line("Resolving deltas: 100% (250/250), done.").unwrap();
        assert_eq!(ev.stage, "resolving");
        assert_eq!(ev.percent, 100);
    }

    #[test]
    fn parse_checkout() {
        let ev = parse_progress_line("Updating files: 50% (50/100)").unwrap();
        assert_eq!(ev.stage, "checkout");
        assert_eq!(ev.percent, 50);
    }

    #[test]
    fn parse_unrecognized_returns_none() {
        assert!(parse_progress_line("warning: redirecting to https://...").is_none());
    }

    #[test]
    fn parse_redacts_token_in_raw_line() {
        let ev = parse_progress_line(
            "Cloning into 'foo'... ghp_abcdefghij1234567890ABCDEFGHIJ123456", // allow-token-pattern: 测试样本
        )
        .unwrap();
        assert!(!ev.raw_line.contains("ghp_"));
    }

    #[test]
    fn split_lines_handles_carriage_returns() {
        let input = "Counting objects: 50%\rCounting objects: 100%, done.\nResolving deltas: 5%";
        let out: Vec<&str> = split_progress_lines(input).collect();
        assert!(out.iter().any(|l| l.contains("50%")));
        assert!(out.iter().any(|l| l.contains("100%")));
        assert!(out.iter().any(|l| l.contains("Resolving deltas")));
    }

    #[test]
    fn shell_escape_handles_single_quotes() {
        assert_eq!(shell_escape("normal"), "normal");
        assert_eq!(shell_escape("it's"), "it'\\''s");
    }

    #[test]
    fn askpass_guard_creates_and_removes_script() {
        let cred = CredentialInjection {
            username: "alice".to_string(),
            token: "ghp_test_token".to_string(),
        };
        let path = {
            let guard = AskpassGuard::create(&cred).unwrap();
            assert!(guard.script_path().exists());
            guard.script_path().to_path_buf()
        };
        assert!(!path.exists(), "drop 后脚本应被删除");
    }

    #[test]
    fn askpass_guard_writes_no_token_in_filename() {
        let cred = CredentialInjection {
            username: "x".to_string(),
            token: "ghp_secret_value_xyz".to_string(),
        };
        let guard = AskpassGuard::create(&cred).unwrap();
        let name = guard
            .script_path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        assert!(!name.contains("secret"));
        assert!(!name.contains("ghp_"));
    }

    // -------------------------------------------------------------------
    // T117 — 启动期清理残留 askpass 脚本
    // -------------------------------------------------------------------

    /// 验收标准（T117）：清理只删本应用前缀的 askpass 脚本，
    /// 其它临时文件原样保留，并返回准确的删除计数。
    #[test]
    fn cleanup_removes_only_gitview_askpass_scripts() {
        let dir = tempfile::tempdir().unwrap();
        // 模拟崩溃残留的 askpass 脚本（.sh / .bat 两类扩展名都覆盖）
        std::fs::write(dir.path().join("gitview-askpass-aaa.sh"), "x").unwrap();
        std::fs::write(dir.path().join("gitview-askpass-bbb.bat"), "x").unwrap();
        // 无关文件：必须保留，验证前缀匹配不会误伤
        std::fs::write(dir.path().join("unrelated.tmp"), "x").unwrap();

        let removed = cleanup_orphan_askpass_scripts(dir.path());

        assert_eq!(removed, 2);
        assert!(
            dir.path().join("unrelated.tmp").exists(),
            "无关文件不应被删除"
        );
        assert!(!dir.path().join("gitview-askpass-aaa.sh").exists());
        assert!(!dir.path().join("gitview-askpass-bbb.bat").exists());
    }

    /// 目录不存在时返回 0 而非 panic：回扫在空白 / 异常环境也要安全。
    #[test]
    fn cleanup_on_missing_dir_returns_zero() {
        let removed = cleanup_orphan_askpass_scripts(Path::new("/no/such/gitview/tmp/dir"));
        assert_eq!(removed, 0);
    }

    // -------------------------------------------------------------------
    // T077 — 写入操作的防御性测试（不依赖真实 git 子进程）
    // -------------------------------------------------------------------

    /// 验收标准（T077）：commit message 为空时立即返回 Internal 错误，
    /// 不写入临时文件、不调用 git 子进程。
    #[tokio::test]
    async fn commit_rejects_empty_message_without_invoking_git() {
        let svc = GitCliService::with_path(PathBuf::from("git"));
        let tmp = std::env::temp_dir();
        let err = svc.commit(&tmp, "   \n  ", None).await.unwrap_err();
        assert!(matches!(err, GitViewError::Internal(_)));
        // 临时文件目录中不应留下 COMMIT_GITVIEW
        assert!(!tmp.join(".git").join("COMMIT_GITVIEW").exists());
    }

    /// 验收标准（T077 + Principle III）：discard_changes 在 confirmed = false
    /// 时必须立即返回 UserCancelled，作为对前端 ConfirmDangerDialog 的双重防御。
    #[tokio::test]
    async fn discard_without_confirmed_returns_user_cancelled() {
        let svc = GitCliService::with_path(PathBuf::from("git"));
        let err = svc
            .discard_changes(Path::new("/nonexistent"), &["a.txt"], false)
            .await
            .unwrap_err();
        assert!(matches!(err, GitViewError::UserCancelled));
    }

    /// 验收标准（T077）：discard_changes 在 files 为空时直接成功，
    /// 不会触发任何 git 调用（避免对整个工作区误操作）。
    #[tokio::test]
    async fn discard_with_empty_files_is_noop() {
        let svc = GitCliService::with_path(PathBuf::from("git"));
        let res = svc
            .discard_changes(Path::new("/nonexistent"), &[], true)
            .await;
        assert!(res.is_ok());
    }

    // -------------------------------------------------------------------
    // T078 — 网络命令的 stderr 映射测试
    // -------------------------------------------------------------------

    #[test]
    fn map_network_failure_dns_error() {
        let err = map_network_failure(
            "fetch",
            "fatal: unable to access 'https://x.example/': Could not resolve host: x.example",
        );
        assert!(matches!(err, GitViewError::Network(_)));
    }

    #[test]
    fn map_network_failure_auth_to_forbidden() {
        let err = map_network_failure("push", "remote: Permission denied to user 'foo'.");
        assert!(matches!(err, GitViewError::Forbidden));
    }

    #[test]
    fn map_network_failure_timeout() {
        let err = map_network_failure("fetch", "Connection timed out after 10000 ms");
        assert!(matches!(err, GitViewError::Network(_)));
    }

    #[test]
    fn map_network_failure_fallthrough_is_git_command() {
        let err = map_network_failure("pull", "some unknown failure");
        assert!(matches!(err, GitViewError::GitCommand(_)));
    }

    #[test]
    fn classify_clone_failure_detects_ssh_publickey() {
        let tail = vec![
            "git@github.com: Permission denied (publickey).".to_string(),
            "fatal: Could not read from remote repository.".to_string(),
        ];
        let GitViewError::GitCommand(msg) = classify_clone_failure(128, &tail) else {
            panic!("应分类为 GitCommand");
        };
        assert!(msg.contains("SSH 认证失败"));
    }

    #[test]
    fn classify_clone_failure_detects_host_key() {
        let tail = vec!["Host key verification failed.".to_string()];
        let GitViewError::GitCommand(msg) = classify_clone_failure(128, &tail) else {
            panic!("应分类为 GitCommand");
        };
        assert!(msg.contains("SSH 认证失败"));
    }

    #[test]
    fn classify_clone_failure_generic_fallthrough() {
        let tail = vec!["fatal: repository not found".to_string()];
        let GitViewError::GitCommand(msg) = classify_clone_failure(128, &tail) else {
            panic!("应分类为 GitCommand");
        };
        assert!(!msg.contains("SSH 认证失败"));
        assert!(msg.contains("128"));
    }

    // -------------------------------------------------------------------
    // T100 — set_git_path 路径校验
    // -------------------------------------------------------------------

    /// 验收标准（T100）：set_git_path 收到不存在的路径时返回 GitNotFound,
    /// 无需真实 git 子进程即可判定（is_file 检查先于命令执行）。
    #[tokio::test]
    async fn set_git_path_rejects_nonexistent() {
        let err = GitCliService::set_git_path(PathBuf::from("/nonexistent/git/binary"))
            .await
            .unwrap_err();
        assert!(matches!(err, GitViewError::GitNotFound));
    }

    // -------------------------------------------------------------------
    // 失败清理：保留用户预建目录 vs 整删
    // -------------------------------------------------------------------

    /// preserve=true：只清空目录内容、保留用户预先建好的目录本身。
    #[test]
    fn cleanup_preserve_keeps_dir_removes_contents() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("repo");
        std::fs::create_dir(&target).unwrap();
        std::fs::write(target.join("file.txt"), "x").unwrap();
        std::fs::create_dir(target.join("sub")).unwrap();

        cleanup_partial_clone(&target, true);

        assert!(target.exists(), "预建目录本身应保留");
        assert!(
            std::fs::read_dir(&target).unwrap().next().is_none(),
            "目录内容应被清空"
        );
    }

    /// preserve=false：clone 自建的目录整体删除。
    #[test]
    fn cleanup_without_preserve_removes_whole_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("repo");
        std::fs::create_dir(&target).unwrap();
        std::fs::write(target.join("file.txt"), "x").unwrap();

        cleanup_partial_clone(&target, false);

        assert!(!target.exists(), "整个目标目录应被删除");
    }
}
