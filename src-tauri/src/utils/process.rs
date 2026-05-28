//! 异步子进程执行封装。
//!
//! 提供统一的命令执行接口，自动注入安全环境变量：
//!   - `LC_ALL=C` — 强制英文输出，便于解析 Git 命令结果
//!   - `GIT_TERMINAL_PROMPT=0` — 禁止 Git 弹出交互式认证窗口
//!
//! 支持超时控制与工作目录指定。

use std::path::Path;
use std::process::Output;
use std::time::Duration;

use tokio::process::Command;

use crate::errors::{GitViewError, Result};

/// 默认命令执行超时时间（秒）
const DEFAULT_TIMEOUT_SECS: u64 = 60;

/// 异步执行外部命令。
///
/// 自动注入 `LC_ALL=C` 与 `GIT_TERMINAL_PROMPT=0` 环境变量，
/// 确保 Git 命令输出为英文且不弹出交互式认证。
///
/// # Arguments
///
/// * `cmd` - 可执行文件路径或名称
/// * `args` - 命令参数列表
/// * `env` - 额外环境变量（key-value 对）
/// * `cwd` - 工作目录（None 则使用当前目录）
///
/// # Returns
///
/// 命令执行的完整输出（stdout + stderr + exit code）。
///
/// # Errors
///
/// - 命令不存在或无法启动 → `GitCommand`
/// - 执行超时 → `GitCommand("命令执行超时")`
pub async fn run_command(
    cmd: &str,
    args: &[&str],
    env: &[(&str, &str)],
    cwd: Option<&Path>,
) -> Result<Output> {
    run_command_with_timeout(cmd, args, env, cwd, DEFAULT_TIMEOUT_SECS).await
}

/// 带自定义超时的异步命令执行。
///
/// # Arguments
///
/// * `timeout_secs` - 超时秒数，超时后子进程将被终止
///
/// 其他参数同 `run_command`。
pub async fn run_command_with_timeout(
    cmd: &str,
    args: &[&str],
    env: &[(&str, &str)],
    cwd: Option<&Path>,
    timeout_secs: u64,
) -> Result<Output> {
    let mut command = Command::new(cmd);

    // 添加命令参数
    command.args(args);

    // 注入基础环境变量（Git 安全配置）
    command.env("LC_ALL", "C");
    command.env("GIT_TERMINAL_PROMPT", "0");

    // 注入用户指定的额外环境变量
    for (key, value) in env {
        command.env(key, value);
    }

    // 设置工作目录
    if let Some(dir) = cwd {
        command.current_dir(dir);
    }

    // 带超时执行
    let result = tokio::time::timeout(Duration::from_secs(timeout_secs), command.output()).await;

    match result {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(e)) => Err(GitViewError::GitCommand(format!(
            "命令 '{cmd}' 执行失败: {e}"
        ))),
        Err(_) => Err(GitViewError::GitCommand(format!(
            "命令 '{cmd}' 执行超时（{timeout_secs}秒）"
        ))),
    }
}

// =====================================================================
// 单元测试
// =====================================================================

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    /// 测试：执行简单命令（echo）
    #[tokio::test]
    async fn test_run_command_echo() {
        let output = run_command("echo", &["hello"], &[], None).await.unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("hello"));
    }

    /// 测试：不存在的命令返回错误
    #[tokio::test]
    async fn test_run_command_not_found() {
        let result = run_command("nonexistent_command_xyz_12345", &[], &[], None).await;
        assert!(result.is_err());
    }
}
