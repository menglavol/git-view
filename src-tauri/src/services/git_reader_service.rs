//! Git 仓库只读服务。
//!
//! 提供仓库状态解析、分支列表、提交历史、diff 查看等只读操作。
//! 所有方法不修改仓库内容，安全用于后台轮询刷新。
//!
//! 公共函数：
//!   - `status` — 解析 `git status --porcelain=v2 --branch -z` 输出
//!   - `list_branches` — 解析 `git branch --all --format=...`
//!   - `log` — 解析 `git log --pretty=format:...`
//!   - `diff` — 调用 `git diff` 返回文本差异
//!   - `derive_repository_status` — 把 GitStatus 映射到 6 个 RepositoryStatus 之一

use std::path::Path;

use chrono::{DateTime, Utc};

use crate::errors::{GitViewError, Result};
use crate::models::git::{Branch, CommitInfo, FileChange, FileStatus, GitStatus};
use crate::models::repository::RepositoryStatus;
use crate::utils::process::run_command;

// =====================================================================
// 工作区状态（git status）
// =====================================================================

/// 解析工作区状态。
///
/// 内部调用 `git status --porcelain=v2 --branch -z`，解析输出得到当前分支、
/// upstream、ahead/behind、所有文件变更。
pub async fn status(repo_path: &Path) -> Result<GitStatus> {
    let output = run_command(
        "git",
        &["status", "--porcelain=v2", "--branch", "-z"],
        &[],
        Some(repo_path),
    )
    .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitViewError::GitCommand(format!(
            "git status 失败：{stderr}"
        )));
    }

    let raw = String::from_utf8_lossy(&output.stdout);
    Ok(parse_porcelain_v2(&raw))
}

// =====================================================================
// 分支列表（git branch）
// =====================================================================

/// 列出所有分支（含远端追踪分支）。
///
/// 输出格式由 `--format` 指定，字段间用 `|` 分隔：name | upstream | HEAD 标记。
/// 当前 V1 阶段实现简化为返回基础字段，ahead/behind 留 0；US5 详情页若需要
/// 精确数字可扩展为额外 `git rev-list` 调用。
pub async fn list_branches(repo_path: &Path) -> Result<Vec<Branch>> {
    let output = run_command(
        "git",
        &[
            "branch",
            "--all",
            "--format=%(refname:short)|%(upstream:short)|%(HEAD)",
        ],
        &[],
        Some(repo_path),
    )
    .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitViewError::GitCommand(format!(
            "git branch 失败：{stderr}"
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut branches = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.splitn(3, '|').collect();
        if parts.is_empty() {
            continue;
        }
        let name = parts[0].trim().to_string();
        let upstream_raw = parts.get(1).copied().unwrap_or("").trim();
        let head_marker = parts.get(2).copied().unwrap_or("").trim();
        // remotes/origin/xxx 形式视为远端分支
        let is_remote = name.starts_with("remotes/") || name.contains('/');
        branches.push(Branch {
            name,
            is_current: head_marker == "*",
            is_remote,
            upstream: if upstream_raw.is_empty() {
                None
            } else {
                Some(upstream_raw.to_string())
            },
            ahead: 0,
            behind: 0,
            last_commit_short: None,
        });
    }

    Ok(branches)
}

// =====================================================================
// 提交历史（git log）
// =====================================================================

/// 分页查询提交历史。
///
/// 字段顺序：`%H | %h | %an | %ae | %aI(ISO 8601) | %s | %P`，
/// 字段分隔符为 `\x1f`（ASCII Unit Separator）避免与提交消息内容冲突。
pub async fn log(repo_path: &Path, page: u32, page_size: u32) -> Result<Vec<CommitInfo>> {
    let offset = page.saturating_mul(page_size);
    let output = run_command(
        "git",
        &[
            "log",
            "--pretty=format:%H%x1f%h%x1f%an%x1f%ae%x1f%aI%x1f%s%x1f%P",
            &format!("-n{page_size}"),
            &format!("--skip={offset}"),
        ],
        &[],
        Some(repo_path),
    )
    .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitViewError::GitCommand(format!("git log 失败：{stderr}")));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut commits = Vec::new();
    for line in stdout.lines() {
        let fields: Vec<&str> = line.split('\x1f').collect();
        if fields.len() < 6 {
            continue;
        }
        // map_or_else 比 .map(...).unwrap_or_else(...) 更直白
        let authored_at = DateTime::parse_from_rfc3339(fields[4])
            .map_or_else(|_| Utc::now(), |dt| dt.with_timezone(&Utc));
        let parent_shas: Vec<String> = if fields.len() > 6 && !fields[6].is_empty() {
            fields[6].split(' ').map(String::from).collect()
        } else {
            Vec::new()
        };
        commits.push(CommitInfo {
            sha: fields[0].to_string(),
            short_sha: fields[1].to_string(),
            summary: fields[5].to_string(),
            message: fields[5].to_string(),
            author_name: fields[2].to_string(),
            author_email: fields[3].to_string(),
            authored_at,
            parent_shas,
        });
    }
    Ok(commits)
}

// =====================================================================
// Diff 查看
// =====================================================================

/// `diff` 返回结果（V1 简化为纯文本 diff）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffResult {
    /// diff 文本内容
    pub text: String,
    /// 是否被截断（超过 1MB 时仅返回前缀）
    pub truncated: bool,
}

/// 查看文件 diff。
///
/// - `file = None`：查看工作区所有变更
/// - `cached = true`：查看暂存区相对 HEAD 的差异
/// - 大文件保护：输出超过 1MB 时截断并标记
pub async fn diff(repo_path: &Path, file: Option<&str>, cached: bool) -> Result<DiffResult> {
    const MAX_DIFF_BYTES: usize = 1024 * 1024;

    let mut args: Vec<&str> = vec!["diff"];
    if cached {
        args.push("--cached");
    }
    args.push("--");
    if let Some(f) = file {
        args.push(f);
    }

    let output = run_command("git", &args, &[], Some(repo_path)).await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitViewError::GitCommand(format!("git diff 失败：{stderr}")));
    }

    let raw = output.stdout;
    let (text, truncated) = if raw.len() > MAX_DIFF_BYTES {
        (
            String::from_utf8_lossy(&raw[..MAX_DIFF_BYTES]).to_string(),
            true,
        )
    } else {
        (String::from_utf8_lossy(&raw).to_string(), false)
    };

    Ok(DiffResult { text, truncated })
}

// =====================================================================
// 状态推导：GitStatus → RepositoryStatus
// =====================================================================

/// 将 `GitStatus` 映射到单一 `RepositoryStatus` 枚举值。
///
/// 优先级：
///   1. 路径不存在 / 无 upstream / detached → Unknown
///   2. 含冲突文件 → Diverged（冲突意味着分叉）
///   3. ahead > 0 && behind > 0 → Diverged
///   4. 有未提交变更 → Dirty
///   5. ahead > 0 → Ahead
///   6. behind > 0 → Behind
///   7. 其他 → Clean
#[must_use]
pub fn derive_repository_status(path: &Path, status: &GitStatus) -> RepositoryStatus {
    if !path.exists() {
        return RepositoryStatus::Unknown;
    }
    // 含冲突的文件：直接视为已分叉，需要用户介入
    let has_conflict = status
        .changes
        .iter()
        .any(|c| matches!(c.status, FileStatus::Conflicted));
    if has_conflict {
        return RepositoryStatus::Diverged;
    }
    // detached HEAD：无当前分支
    if status.current_branch.is_none() {
        return RepositoryStatus::Unknown;
    }
    // 有未提交变更：Dirty 优先级高于 ahead/behind（先解决脏工作区）
    if !status.is_clean {
        return RepositoryStatus::Dirty;
    }
    // ahead + behind：分叉
    if status.ahead > 0 && status.behind > 0 {
        return RepositoryStatus::Diverged;
    }
    if status.ahead > 0 {
        return RepositoryStatus::Ahead;
    }
    if status.behind > 0 {
        return RepositoryStatus::Behind;
    }
    // 没有 upstream：状态不可知
    if status.upstream.is_none() {
        return RepositoryStatus::Unknown;
    }
    RepositoryStatus::Clean
}

// =====================================================================
// porcelain=v2 解析
// =====================================================================

/// 解析 `git status --porcelain=v2 --branch -z` 输出。
///
/// 输出包含 `# branch.head`、`# branch.upstream`、`# branch.ab +A -B` 等头部
/// 行，以及每个变更条目（1/2/u/?/!）。条目间用 NUL 分隔（`-z`）。
fn parse_porcelain_v2(raw: &str) -> GitStatus {
    let mut current_branch: Option<String> = None;
    let mut upstream: Option<String> = None;
    let mut ahead: u32 = 0;
    let mut behind: u32 = 0;
    let mut changes: Vec<FileChange> = Vec::new();

    for entry in raw.split('\0') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }

        // 头部 # 行
        if let Some(rest) = entry.strip_prefix("# branch.head ") {
            // detached HEAD 时输出 `(detached)`
            if rest != "(detached)" {
                current_branch = Some(rest.to_string());
            }
        } else if let Some(rest) = entry.strip_prefix("# branch.upstream ") {
            upstream = Some(rest.to_string());
        } else if let Some(rest) = entry.strip_prefix("# branch.ab ") {
            // 格式 `+N -M`
            for part in rest.split_whitespace() {
                if let Some(n) = part.strip_prefix('+') {
                    ahead = n.parse().unwrap_or(0);
                } else if let Some(n) = part.strip_prefix('-') {
                    behind = n.parse().unwrap_or(0);
                }
            }
        } else if entry.starts_with("# ") {
            // 其他头部行（branch.oid / stash.count 等），暂不处理
        } else if let Some(rest) = entry.strip_prefix("1 ") {
            // Ordinary changed entry: 1 <XY> <sub> <mH> <mI> <mW> <hH> <hI> <path>
            if let Some(change) = parse_ordinary_entry(rest, false) {
                changes.push(change);
            }
        } else if let Some(rest) = entry.strip_prefix("2 ") {
            // Renamed entry: 与 1 类似但 path 含 \t 分隔的 old_path
            if let Some(change) = parse_ordinary_entry(rest, true) {
                changes.push(change);
            }
        } else if let Some(rest) = entry.strip_prefix("u ") {
            // Unmerged entry: u <XY> <sub> <m1> <m2> <m3> <mW> <h1> <h2> <h3> <path>
            if let Some(path_part) = rest.splitn(10, ' ').nth(9) {
                changes.push(FileChange {
                    path: path_part.to_string(),
                    old_path: None,
                    status: FileStatus::Conflicted,
                    staged: false,
                });
            }
        } else if let Some(rest) = entry.strip_prefix("? ") {
            // Untracked
            changes.push(FileChange {
                path: rest.to_string(),
                old_path: None,
                status: FileStatus::Untracked,
                staged: false,
            });
        } else if let Some(rest) = entry.strip_prefix("! ") {
            // Ignored
            changes.push(FileChange {
                path: rest.to_string(),
                old_path: None,
                status: FileStatus::Ignored,
                staged: false,
            });
        }
    }

    let is_clean = changes.is_empty();

    GitStatus {
        current_branch,
        upstream,
        ahead,
        behind,
        changes,
        is_clean,
    }
}

/// 解析 `1`/`2` 起始的普通变更条目。
fn parse_ordinary_entry(rest: &str, renamed: bool) -> Option<FileChange> {
    // 字段以单空格分隔；前 7 字段为元数据，path 是第 8 字段（含可能的空格）
    let parts: Vec<&str> = rest.splitn(8, ' ').collect();
    if parts.len() < 8 {
        return None;
    }
    let xy = parts[0]; // 形如 "M.", ".M", "MM"
    let path_part = parts[7];

    // 解析 XY 标志：第 1 字符是 staged 状态，第 2 字符是 worktree 状态
    let xy_bytes = xy.as_bytes();
    let staged_char = xy_bytes.first().copied().unwrap_or(b'.');
    let work_char = xy_bytes.get(1).copied().unwrap_or(b'.');
    let staged = staged_char != b'.';

    let status = if work_char == b'M' || staged_char == b'M' {
        FileStatus::Modified
    } else if work_char == b'A' || staged_char == b'A' {
        FileStatus::Added
    } else if work_char == b'D' || staged_char == b'D' {
        FileStatus::Deleted
    } else if renamed || work_char == b'R' || staged_char == b'R' {
        FileStatus::Renamed
    } else {
        FileStatus::Modified
    };

    // 重命名时 path 形如 `<new>\t<old>`
    let (path, old_path) = if renamed {
        path_part.find('\t').map_or_else(
            || (path_part.to_string(), None),
            |idx| {
                (
                    path_part[..idx].to_string(),
                    Some(path_part[idx + 1..].to_string()),
                )
            },
        )
    } else {
        (path_part.to_string(), None)
    };

    Some(FileChange {
        path,
        old_path,
        status: if staged { FileStatus::Staged } else { status },
        staged,
    })
}

// =====================================================================
// 单元测试：覆盖 derive_repository_status 的 8 种典型场景
// =====================================================================
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    /// 构造一个最简 GitStatus，按需在调用处覆盖字段
    fn make_status(
        branch: Option<&str>,
        upstream: Option<&str>,
        ahead: u32,
        behind: u32,
        changes: Vec<FileChange>,
    ) -> GitStatus {
        let is_clean = changes.is_empty();
        GitStatus {
            current_branch: branch.map(String::from),
            upstream: upstream.map(String::from),
            ahead,
            behind,
            changes,
            is_clean,
        }
    }

    #[test]
    fn derive_status_clean() {
        let tmp = tempfile::tempdir().unwrap();
        let s = make_status(Some("main"), Some("origin/main"), 0, 0, vec![]);
        assert_eq!(
            derive_repository_status(tmp.path(), &s),
            RepositoryStatus::Clean
        );
    }

    #[test]
    fn derive_status_dirty() {
        let tmp = tempfile::tempdir().unwrap();
        let s = make_status(
            Some("main"),
            Some("origin/main"),
            0,
            0,
            vec![FileChange {
                path: "a.txt".to_string(),
                old_path: None,
                status: FileStatus::Modified,
                staged: false,
            }],
        );
        assert_eq!(
            derive_repository_status(tmp.path(), &s),
            RepositoryStatus::Dirty
        );
    }

    #[test]
    fn derive_status_ahead() {
        let tmp = tempfile::tempdir().unwrap();
        let s = make_status(Some("main"), Some("origin/main"), 2, 0, vec![]);
        assert_eq!(
            derive_repository_status(tmp.path(), &s),
            RepositoryStatus::Ahead
        );
    }

    #[test]
    fn derive_status_behind() {
        let tmp = tempfile::tempdir().unwrap();
        let s = make_status(Some("main"), Some("origin/main"), 0, 3, vec![]);
        assert_eq!(
            derive_repository_status(tmp.path(), &s),
            RepositoryStatus::Behind
        );
    }

    #[test]
    fn derive_status_diverged() {
        let tmp = tempfile::tempdir().unwrap();
        let s = make_status(Some("main"), Some("origin/main"), 1, 2, vec![]);
        assert_eq!(
            derive_repository_status(tmp.path(), &s),
            RepositoryStatus::Diverged
        );
    }

    #[test]
    fn derive_status_conflict_takes_priority() {
        let tmp = tempfile::tempdir().unwrap();
        let s = make_status(
            Some("main"),
            Some("origin/main"),
            5,
            0,
            vec![FileChange {
                path: "conflict.txt".to_string(),
                old_path: None,
                status: FileStatus::Conflicted,
                staged: false,
            }],
        );
        // 即使 ahead > 0，冲突优先级更高，返回 Diverged
        assert_eq!(
            derive_repository_status(tmp.path(), &s),
            RepositoryStatus::Diverged
        );
    }

    #[test]
    fn derive_status_no_upstream() {
        let tmp = tempfile::tempdir().unwrap();
        // 有分支但无 upstream
        let s = make_status(Some("main"), None, 0, 0, vec![]);
        assert_eq!(
            derive_repository_status(tmp.path(), &s),
            RepositoryStatus::Unknown
        );
    }

    #[test]
    fn derive_status_detached_head() {
        let tmp = tempfile::tempdir().unwrap();
        // 无当前分支即 detached HEAD
        let s = make_status(None, None, 0, 0, vec![]);
        assert_eq!(
            derive_repository_status(tmp.path(), &s),
            RepositoryStatus::Unknown
        );
    }
}
