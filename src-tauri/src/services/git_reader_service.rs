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
use crate::models::git::{
    Branch, CommitDetail, CommitFile, CommitFileStatus, CommitInfo, CommitStats, FileChange,
    FileStatus, GitStatus,
};
use crate::models::repository::RepositoryStatus;
use crate::services::provider::truncate_file_diff;
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
// 单提交详情（git show）
// =====================================================================

/// 获取单个提交的详情：元信息 + 改动文件 + 每文件 diff。
///
/// 两次 `git show`：`-s` 取格式化元信息（含完整正文 %B）；`--patch` 取整体
/// diff，再按 `diff --git` 边界切块为各文件，并从块内 +/- 行统计增删数。
pub async fn commit_detail(repo_path: &Path, sha: &str) -> Result<CommitDetail> {
    // 1) 元信息：字段用 \x1f 分隔，%B（完整正文，可能含换行）置于末尾
    let meta_out = run_command(
        "git",
        &[
            "show",
            "-s",
            "--format=%H%x1f%h%x1f%an%x1f%ae%x1f%aI%x1f%cn%x1f%ce%x1f%cI%x1f%P%x1f%B",
            sha,
        ],
        &[],
        Some(repo_path),
    )
    .await?;
    if !meta_out.status.success() {
        let stderr = String::from_utf8_lossy(&meta_out.stderr);
        return Err(GitViewError::GitCommand(format!("git show 失败：{stderr}")));
    }
    let meta_raw = String::from_utf8_lossy(&meta_out.stdout);
    // splitn(10)：前 9 个字段无换行，第 10 段为可能含换行的完整正文
    let fields: Vec<&str> = meta_raw.splitn(10, '\x1f').collect();
    if fields.len() < 10 {
        return Err(GitViewError::GitCommand("解析提交元信息失败".to_string()));
    }
    let authored_at = DateTime::parse_from_rfc3339(fields[4])
        .map_or_else(|_| Utc::now(), |dt| dt.with_timezone(&Utc));
    let committed_at = DateTime::parse_from_rfc3339(fields[7])
        .ok()
        .map(|dt| dt.with_timezone(&Utc));
    let parent_shas: Vec<String> = if fields[8].trim().is_empty() {
        Vec::new()
    } else {
        fields[8].split(' ').map(String::from).collect()
    };

    // 2) 整体 diff，按文件切块
    let patch_out = run_command(
        "git",
        &["show", "--format=", "--patch", sha],
        &[],
        Some(repo_path),
    )
    .await?;
    if !patch_out.status.success() {
        let stderr = String::from_utf8_lossy(&patch_out.stderr);
        return Err(GitViewError::GitCommand(format!(
            "git show diff 失败：{stderr}"
        )));
    }
    let patch_raw = String::from_utf8_lossy(&patch_out.stdout);
    let files = parse_commit_files(&patch_raw);

    // 汇总增删行（各文件之和）
    let (additions, deletions) = files.iter().fold((0u32, 0u32), |(a, d), f| {
        (a + f.additions.unwrap_or(0), d + f.deletions.unwrap_or(0))
    });

    Ok(CommitDetail {
        sha: fields[0].to_string(),
        short_sha: fields[1].to_string(),
        message: fields[9].trim_end().to_string(),
        author_name: fields[2].to_string(),
        author_email: fields[3].to_string(),
        authored_at,
        committer_name: opt_string(fields[5]),
        committer_email: opt_string(fields[6]),
        committed_at,
        parent_shas,
        html_url: None,
        stats: Some(CommitStats {
            additions,
            deletions,
            total: additions + deletions,
        }),
        files,
    })
}

/// 空串转 None：用于可空元信息字段（提交者姓名 / 邮箱）。
fn opt_string(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

/// 把 `git show --patch` 的整体 diff 按 `diff --git` 边界切块为各文件。
fn parse_commit_files(patch: &str) -> Vec<CommitFile> {
    let mut files = Vec::new();
    let mut current: Option<FileAccum> = None;
    for line in patch.lines() {
        if line.starts_with("diff --git ") {
            // 遇到新文件块，先收尾上一个
            if let Some(acc) = current.take() {
                files.push(acc.into_file());
            }
            current = Some(FileAccum::new(line));
        } else if let Some(acc) = current.as_mut() {
            acc.feed(line);
        }
    }
    if let Some(acc) = current.take() {
        files.push(acc.into_file());
    }
    files
}

/// 单文件 diff 块的累积器：边读边解析状态 / 路径 / 增删行。
struct FileAccum {
    lines: Vec<String>,
    status: CommitFileStatus,
    old_path: Option<String>,
    new_path: Option<String>,
    additions: u32,
    deletions: u32,
}

impl FileAccum {
    /// 从 `diff --git a/X b/Y` 行初始化（路径含空格的罕见情况按 V1 简化处理）。
    fn new(header_line: &str) -> Self {
        let (old, new) = parse_diff_git_paths(header_line);
        Self {
            lines: vec![header_line.to_string()],
            status: CommitFileStatus::Modified,
            old_path: old,
            new_path: new,
            additions: 0,
            deletions: 0,
        }
    }

    /// 累积块内一行，并按块头标志推断状态、按 +/- 前缀统计增删。
    fn feed(&mut self, line: &str) {
        self.lines.push(line.to_string());
        if line.starts_with("new file mode") {
            self.status = CommitFileStatus::Added;
        } else if line.starts_with("deleted file mode") {
            self.status = CommitFileStatus::Deleted;
        } else if let Some(p) = line.strip_prefix("rename from ") {
            self.status = CommitFileStatus::Renamed;
            self.old_path = Some(p.to_string());
        } else if let Some(p) = line.strip_prefix("rename to ") {
            self.status = CommitFileStatus::Renamed;
            self.new_path = Some(p.to_string());
        } else if line.starts_with("+++") || line.starts_with("---") {
            // diff 文件头（+++/---），不计入增删
        } else if line.starts_with('+') {
            self.additions += 1;
        } else if line.starts_with('-') {
            self.deletions += 1;
        }
    }

    /// 收尾为 `CommitFile`（diff 文本按上限截断）。
    fn into_file(self) -> CommitFile {
        let (diff, truncated) = truncate_file_diff(&self.lines.join("\n"));
        // 仅重命名保留旧路径
        let old_path = if self.status == CommitFileStatus::Renamed {
            self.old_path
        } else {
            None
        };
        CommitFile {
            path: self.new_path.unwrap_or_default(),
            old_path,
            status: self.status,
            additions: Some(self.additions),
            deletions: Some(self.deletions),
            diff: Some(diff),
            truncated,
        }
    }
}

/// 从 `diff --git a/<old> b/<new>` 行解析旧 / 新路径。
fn parse_diff_git_paths(line: &str) -> (Option<String>, Option<String>) {
    let rest = line.strip_prefix("diff --git ").unwrap_or("");
    // 以 " b/" 分隔 a 段与 b 段（路径含空格的极端情况按 V1 简化忽略）
    rest.find(" b/").map_or((None, None), |idx| {
        let old = rest[..idx].strip_prefix("a/").map(String::from);
        let new = Some(rest[idx + 3..].to_string());
        (old, new)
    })
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

    // ===================================================================
    // T092 — porcelain v2 解析单元测试
    // ===================================================================
    //
    // 覆盖头部行(branch.head / branch.upstream / branch.ab)、ordinary 条目、
    // rename 条目、untracked 与 unmerged(冲突)。所有样本以 NUL 分隔,
    // 模拟 `git status --porcelain=v2 --branch -z` 真实输出格式。

    /// 验收(T092):头部行解析正确,文件变更入列且 staged 标志正确。
    ///
    /// 注:当前 `parse_ordinary_entry` 实现对已暂存文件统一返回
    /// `FileStatus::Staged`(原始变更类型由 `staged` 布尔字段补充语义)。
    #[test]
    fn parse_porcelain_v2_branch_and_changes() {
        // 构造样本:main 分支、领先 2 落后 1、3 个文件变更
        // 字段顺序: 1 <XY> <sub> <mH> <mI> <mW> <hH> <hI> <path>
        let raw = concat!(
            "# branch.oid abc123\0",
            "# branch.head main\0",
            "# branch.upstream origin/main\0",
            "# branch.ab +2 -1\0",
            // 普通修改 + 已暂存(staged 字符 'M',工作区 '.')
            "1 M. N... 100644 100644 100644 h1 h2 src/a.rs\0",
            // 工作区修改未暂存(staged '.',工作区 'M')
            "1 .M N... 100644 100644 100644 h3 h4 src/b.rs\0",
            // 已暂存的删除(staged 'D')
            "1 D. N... 100644 100644 100644 h5 h6 src/c.rs\0",
        );
        let s = parse_porcelain_v2(raw);
        assert_eq!(s.current_branch.as_deref(), Some("main"));
        assert_eq!(s.upstream.as_deref(), Some("origin/main"));
        assert_eq!(s.ahead, 2);
        assert_eq!(s.behind, 1);
        assert_eq!(s.changes.len(), 3);
        assert!(!s.is_clean);

        // 第一个文件:暂存修改 → staged=true, status=Staged
        assert_eq!(s.changes[0].path, "src/a.rs");
        assert!(s.changes[0].staged);
        assert_eq!(s.changes[0].status, FileStatus::Staged);

        // 第二个文件:工作区修改未暂存 → staged=false, status=Modified
        assert_eq!(s.changes[1].path, "src/b.rs");
        assert!(!s.changes[1].staged);
        assert_eq!(s.changes[1].status, FileStatus::Modified);

        // 第三个文件:已暂存删除 → staged=true, status=Staged
        assert_eq!(s.changes[2].path, "src/c.rs");
        assert!(s.changes[2].staged);
        assert_eq!(s.changes[2].status, FileStatus::Staged);
    }

    /// 验收(T092):untracked 与 ignored 条目分类正确。
    #[test]
    fn parse_porcelain_v2_untracked_and_ignored() {
        let raw = concat!(
            "# branch.head main\0",
            "? new-file.txt\0",
            "? temp/draft.md\0",
            "! .DS_Store\0",
        );
        let s = parse_porcelain_v2(raw);
        assert_eq!(s.changes.len(), 3);
        assert_eq!(s.changes[0].status, FileStatus::Untracked);
        assert_eq!(s.changes[0].path, "new-file.txt");
        assert_eq!(s.changes[1].status, FileStatus::Untracked);
        assert_eq!(s.changes[1].path, "temp/draft.md");
        assert_eq!(s.changes[2].status, FileStatus::Ignored);
        assert_eq!(s.changes[2].path, ".DS_Store");
    }

    /// 验收(T092):detached HEAD 时 current_branch 为 None。
    #[test]
    fn parse_porcelain_v2_detached_head() {
        let raw = concat!("# branch.oid abc\0", "# branch.head (detached)\0",);
        let s = parse_porcelain_v2(raw);
        assert!(s.current_branch.is_none());
        assert!(s.is_clean);
    }

    /// 验收(T092):unmerged 条目(冲突)解析为 Conflicted 状态。
    #[test]
    fn parse_porcelain_v2_unmerged_conflict() {
        // unmerged 格式: u <XY> <sub> <m1> <m2> <m3> <mW> <h1> <h2> <h3> <path>
        let raw = concat!(
            "# branch.head main\0",
            "u UU N... 100644 100644 100644 100644 h1 h2 h3 conflict.txt\0",
        );
        let s = parse_porcelain_v2(raw);
        assert_eq!(s.changes.len(), 1);
        assert_eq!(s.changes[0].status, FileStatus::Conflicted);
        assert_eq!(s.changes[0].path, "conflict.txt");
    }

    /// 验收(T092):空输入或仅头部时 is_clean = true。
    #[test]
    fn parse_porcelain_v2_clean_when_no_changes() {
        let raw = concat!(
            "# branch.head main\0",
            "# branch.upstream origin/main\0",
            "# branch.ab +0 -0\0",
        );
        let s = parse_porcelain_v2(raw);
        assert!(s.is_clean);
        assert_eq!(s.changes.len(), 0);
        assert_eq!(s.ahead, 0);
        assert_eq!(s.behind, 0);
    }

    /// 验收(T092):空字符串安全降级,不 panic。
    #[test]
    fn parse_porcelain_v2_empty_input_is_safe() {
        let s = parse_porcelain_v2("");
        assert!(s.current_branch.is_none());
        assert!(s.is_clean);
    }
}
