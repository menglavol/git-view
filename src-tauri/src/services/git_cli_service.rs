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
    /// 探测系统 Git 可执行文件。
    ///
    /// 返回包含路径、版本号、`user.name`/`user.email` 的检测结果。
    pub async fn detect() -> Result<GitVersionInfo> {
        let git_path = locate_git_executable().await?;

        let version_output = Command::new(&git_path)
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
        // strip_prefix 返回 None 时回退到 trim 结果；用 unwrap_or 会强制 eager
        // 求值 fallback（虽然这里无副作用），换 unwrap_or_else 与现代约定一致
        let version = trimmed
            .strip_prefix("git version ")
            .unwrap_or(trimmed)
            .to_string();

        let user_name = read_git_config(&git_path, "user.name").await.ok();
        let user_email = read_git_config(&git_path, "user.email").await.ok();

        Ok(GitVersionInfo {
            path: git_path,
            version,
            user_name,
            user_email,
        })
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
    /// 失败或取消时**自动删除半成品 target_path**。
    pub async fn clone_repository<F>(
        &self,
        remote_url: &str,
        target_path: &Path,
        credentials: Option<CredentialInjection>,
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
                cleanup_partial_clone(target_path);
                drop(askpass_guard);
                return Err(GitViewError::UserCancelled);
            }
        };

        let _ = parser_handle.await;

        if !exit_status.success() {
            cleanup_partial_clone(target_path);
            let code = exit_status.code().unwrap_or(-1);
            return Err(GitViewError::GitCommand(format!("git clone 退出码 {code}")));
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
        // .is_ok_and 比 .map(...).unwrap_or(false) 更直白
        if Command::new(path)
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
    let output = Command::new(git_path)
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

fn spawn_progress_reader<F>(child: &mut Child, progress: Arc<F>) -> tokio::task::JoinHandle<()>
where
    F: Fn(CloneProgressEvent) + Send + Sync + 'static,
{
    // 拿不到 stderr（极少见，比如已被消费）时返回一个空 future，等价于 noop
    let Some(stderr) = child.stderr.take() else {
        return tokio::spawn(async {});
    };

    tokio::spawn(async move {
        let mut reader = BufReader::new(stderr);
        let mut chunk = Vec::new();
        loop {
            // Ok(0) 表示 EOF，Err 表示 IO 异常——两者均退出循环
            match reader.read_until(b'\n', &mut chunk).await {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
            let text = String::from_utf8_lossy(&chunk).to_string();
            chunk.clear();
            for line in split_progress_lines(&text) {
                if let Some(ev) = parse_progress_line(line) {
                    (progress)(ev);
                }
            }
        }
    })
}

fn cleanup_partial_clone(target_path: &Path) {
    if target_path.exists() {
        let _ = std::fs::remove_dir_all(target_path);
    }
}

// =====================================================================
// GIT_ASKPASS 临时脚本（T056）
// =====================================================================

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
            dir.join(format!("gitview-askpass-{id}.bat"))
        } else {
            dir.join(format!("gitview-askpass-{id}.sh"))
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
}
