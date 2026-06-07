//! 路径工具函数。
//!
//! 提供跨平台路径操作：规范化、目录创建、Git 仓库检测、安全拼接。
//! 使用 `dunce` crate 在 Windows 上去除 UNC 前缀（`\\?\`），
//! 保证路径在 UI 展示与 Git 命令中的兼容性。

use std::path::{Path, PathBuf};

use crate::errors::{GitViewError, Result};

/// 规范化路径：解析符号链接并去除 Windows UNC 前缀。
///
/// # Arguments
///
/// * `path` - 待规范化的路径
///
/// # Returns
///
/// 规范化后的绝对路径；若路径不存在则返回 `PathMissing` 错误。
pub fn normalize_path(path: &Path) -> Result<PathBuf> {
    dunce::canonicalize(path).map_err(|_| GitViewError::PathMissing(path.display().to_string()))
}

/// 确保目录存在，不存在则递归创建。
///
/// # Arguments
///
/// * `dir` - 目标目录路径
///
/// # Errors
///
/// 当目录创建失败时（权限不足、磁盘满等）返回 `Internal` 错误。
pub fn ensure_dir_exists(dir: &Path) -> Result<()> {
    if !dir.exists() {
        std::fs::create_dir_all(dir).map_err(|e| {
            GitViewError::Internal(format!("无法创建目录 {}: {}", dir.display(), e))
        })?;
    }
    Ok(())
}

/// 检测指定路径是否为 Git 仓库（包含 `.git` 目录或文件）。
///
/// # Arguments
///
/// * `path` - 待检测的目录路径
///
/// # Returns
///
/// `true` 表示该路径是 Git 仓库根目录。
#[must_use]
pub fn is_git_repository(path: &Path) -> bool {
    path.join(".git").exists()
}

/// 安全拼接路径，防止路径遍历攻击。
///
/// 确保拼接后的路径仍在 `base` 目录下，
/// 若 `child` 包含 `..` 等导致逃逸的组件则返回错误。
///
/// # Arguments
///
/// * `base` - 基础目录
/// * `child` - 子路径（相对路径）
///
/// # Errors
///
/// 当拼接结果逃逸出 `base` 目录时返回 `PathConflict` 错误。
pub fn join_safe(base: &Path, child: &str) -> Result<PathBuf> {
    let joined = base.join(child);

    // 规范化后检查是否仍以 base 为前缀
    // 注意：若路径不存在无法 canonicalize，使用逻辑检查
    let normalized = joined.components().fold(PathBuf::new(), |mut acc, comp| {
        match comp {
            std::path::Component::ParentDir => {
                acc.pop();
            }
            std::path::Component::Normal(s) => {
                acc.push(s);
            }
            std::path::Component::RootDir => {
                acc.push(std::path::MAIN_SEPARATOR.to_string());
            }
            _ => {}
        }
        acc
    });

    let base_normalized = base.components().fold(PathBuf::new(), |mut acc, comp| {
        match comp {
            std::path::Component::ParentDir => {
                acc.pop();
            }
            std::path::Component::Normal(s) => {
                acc.push(s);
            }
            std::path::Component::RootDir => {
                acc.push(std::path::MAIN_SEPARATOR.to_string());
            }
            _ => {}
        }
        acc
    });

    if normalized.starts_with(&base_normalized) {
        Ok(joined)
    } else {
        Err(GitViewError::PathConflict(format!(
            "路径 '{}' 逃逸出基础目录 '{}'",
            child,
            base.display()
        )))
    }
}

/// 应用数据根目录：`<data_local_dir>/gitview`（日志与数据库都在此目录下）。
#[must_use]
pub fn app_data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("gitview")
}

/// 日志目录：`<app_data_dir>/logs`（tracing-appender 按天滚动写入此处）。
#[must_use]
pub fn log_dir() -> PathBuf {
    app_data_dir().join("logs")
}

// =====================================================================
// 单元测试
// =====================================================================

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    /// 测试：Git 仓库检测 — 存在 .git 目录
    #[test]
    fn test_is_git_repository_true() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir(tmp.path().join(".git")).unwrap();
        assert!(is_git_repository(tmp.path()));
    }

    /// 测试：Git 仓库检测 — 不存在 .git
    #[test]
    fn test_is_git_repository_false() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(!is_git_repository(tmp.path()));
    }

    /// 测试：安全路径拼接 — 正常子路径
    #[test]
    fn test_join_safe_normal() {
        let base = Path::new("/home/user/projects");
        let result = join_safe(base, "repo/src/main.rs");
        assert!(result.is_ok());
    }

    /// 测试：安全路径拼接 — 路径遍历攻击
    #[test]
    fn test_join_safe_traversal() {
        let base = Path::new("/home/user/projects");
        let result = join_safe(base, "../../etc/passwd");
        assert!(result.is_err());
    }
}
