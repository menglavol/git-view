//! 仓库服务。
//!
//! 提供远程仓库查询（筛选/搜索/收藏）与本地仓库管理（添加/扫描/移除/状态刷新/批量 Fetch）。

use std::collections::HashSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use chrono::Utc;
use rusqlite::{params, params_from_iter, OptionalExtension};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::db::pool::DbPool;
use crate::errors::{GitViewError, Result};
use crate::models::account::GitPlatform;
use crate::models::repository::{
    LocalRepository, RemoteRepository, RepositoryStatus, ScanResult, Visibility,
};
use crate::services::git_reader_service;
use crate::utils::path::is_git_repository;

/// 远程仓库筛选条件。
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteRepoFilter {
    pub account_id: Option<String>,
    #[serde(default)]
    pub platforms: Vec<String>,
    #[serde(default)]
    pub owners: Vec<String>,
    #[serde(default)]
    pub visibilities: Vec<String>,
    #[serde(default)]
    pub only_favorite: bool,
    pub search: Option<String>,
}

/// 查询远程仓库列表。
pub fn list_remote_repositories(
    pool: &DbPool,
    filter: &RemoteRepoFilter,
) -> Result<Vec<RemoteRepository>> {
    pool.with_conn(|conn| {
        let mut sql = String::from(
            "SELECT id, account_id, platform, remote_id, full_name, name, owner,
                    description, visibility, default_branch, html_url, ssh_url,
                    clone_url, is_favorite, last_pushed_at, synced_at
             FROM remote_repositories WHERE 1=1",
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref acc_id) = filter.account_id {
            sql.push_str(" AND account_id = ?");
            param_values.push(Box::new(acc_id.clone()));
        }

        if !filter.platforms.is_empty() {
            let placeholders: Vec<String> = filter
                .platforms
                .iter()
                .enumerate()
                .map(|_| "?".to_string())
                .collect();
            let _ = write!(sql, " AND platform IN ({})", placeholders.join(","));
            for p in &filter.platforms {
                param_values.push(Box::new(p.clone()));
            }
        }

        if !filter.owners.is_empty() {
            let placeholders: Vec<String> = filter
                .owners
                .iter()
                .enumerate()
                .map(|_| "?".to_string())
                .collect();
            let _ = write!(sql, " AND owner IN ({})", placeholders.join(","));
            for o in &filter.owners {
                param_values.push(Box::new(o.clone()));
            }
        }

        if !filter.visibilities.is_empty() {
            let placeholders: Vec<String> = filter
                .visibilities
                .iter()
                .enumerate()
                .map(|_| "?".to_string())
                .collect();
            let _ = write!(sql, " AND visibility IN ({})", placeholders.join(","));
            for v in &filter.visibilities {
                param_values.push(Box::new(v.clone()));
            }
        }

        if filter.only_favorite {
            sql.push_str(" AND is_favorite = 1");
        }

        if let Some(ref keyword) = filter.search {
            if !keyword.trim().is_empty() {
                sql.push_str(
                    " AND (full_name LIKE '%' || ? || '%' OR description LIKE '%' || ? || '%')",
                );
                param_values.push(Box::new(keyword.clone()));
                param_values.push(Box::new(keyword.clone()));
            }
        }

        sql.push_str(" ORDER BY last_pushed_at DESC NULLS LAST, synced_at DESC");

        let params_ref: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(AsRef::as_ref).collect();

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(params_ref), row_to_remote_repo)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    })
}

/// 获取单个远程仓库详情。
pub fn get_remote_repository(pool: &DbPool, repo_id: &str) -> Result<RemoteRepository> {
    pool.with_conn(|conn| get_remote_repository_by_id(conn, repo_id))
}

/// 在已有连接上读取单个远程仓库（供事务内复用）。
pub fn get_remote_repository_by_id(
    conn: &rusqlite::Connection,
    repo_id: &str,
) -> Result<RemoteRepository> {
    conn.query_row(
        "SELECT id, account_id, platform, remote_id, full_name, name, owner,
                description, visibility, default_branch, html_url, ssh_url,
                clone_url, is_favorite, last_pushed_at, synced_at
         FROM remote_repositories WHERE id = ?1",
        [repo_id],
        row_to_remote_repo,
    )
    .map_err(GitViewError::from)
}

/// 切换远程仓库收藏状态。
pub fn toggle_favorite(pool: &DbPool, repo_id: &str) -> Result<bool> {
    pool.with_conn(|conn| {
        let current: i64 = conn
            .query_row(
                "SELECT is_favorite FROM remote_repositories WHERE id = ?1",
                [repo_id],
                |row| row.get(0),
            )
            .map_err(GitViewError::from)?;
        let new_val = i64::from(current == 0);
        conn.execute(
            "UPDATE remote_repositories SET is_favorite = ?1 WHERE id = ?2",
            rusqlite::params![new_val, repo_id],
        )
        .map_err(GitViewError::from)?;
        Ok(new_val == 1)
    })
}

// =====================================================================
// 行映射
// =====================================================================

fn deserialize_platform(s: &str) -> GitPlatform {
    match s {
        "github" => GitPlatform::Github,
        "gitlab" => GitPlatform::Gitlab,
        _ => GitPlatform::Gitee,
    }
}

fn deserialize_visibility(s: &str) -> Visibility {
    match s {
        "public" => Visibility::Public,
        "internal" => Visibility::Internal,
        _ => Visibility::Private,
    }
}

fn row_to_remote_repo(row: &rusqlite::Row<'_>) -> rusqlite::Result<RemoteRepository> {
    use chrono::{DateTime, Utc};

    let platform_str: String = row.get("platform")?;
    let vis_str: String = row.get("visibility")?;
    let synced_str: String = row.get("synced_at")?;
    let pushed_str: Option<String> = row.get("last_pushed_at")?;

    let parse_dt = |s: &str| -> rusqlite::Result<DateTime<Utc>> {
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

    Ok(RemoteRepository {
        id: row.get("id")?,
        account_id: row.get("account_id")?,
        platform: deserialize_platform(&platform_str),
        remote_id: row.get("remote_id")?,
        full_name: row.get("full_name")?,
        name: row.get("name")?,
        owner: row.get("owner")?,
        description: row.get("description")?,
        visibility: deserialize_visibility(&vis_str),
        default_branch: row.get("default_branch")?,
        html_url: row.get("html_url")?,
        ssh_url: row.get("ssh_url")?,
        clone_url: row.get("clone_url")?,
        is_favorite: row.get::<_, i64>("is_favorite")? != 0,
        last_pushed_at: pushed_str.and_then(|s| parse_dt(&s).ok()),
        synced_at: parse_dt(&synced_str)?,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use chrono::Utc;
    use rusqlite::params;

    fn fresh_pool() -> DbPool {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        let _ = tmp.keep();
        let pool = DbPool::new(&path).unwrap();
        crate::db::migrations::run_pending_migrations(&pool).unwrap();
        pool
    }

    fn seed_account(pool: &DbPool, id: &str, platform: &str) {
        let now = Utc::now().to_rfc3339();
        pool.with_conn(|conn| {
            conn.execute(
                "INSERT INTO accounts (id, platform, web_base_url, api_base_url, username,
                    token_key, is_default, enabled, created_at, updated_at)
                 VALUES (?1, ?2, 'https://example.com', 'https://api.example.com', ?3,
                    ?4, 0, 1, ?5, ?5)",
                params![
                    id,
                    platform,
                    format!("user-{id}"),
                    format!("token-{id}"),
                    now
                ],
            )?;
            Ok(())
        })
        .unwrap();
    }

    #[allow(clippy::too_many_arguments)]
    fn seed_repo(
        pool: &DbPool,
        id: &str,
        account_id: &str,
        platform: &str,
        owner: &str,
        name: &str,
        visibility: &str,
        is_favorite: bool,
        description: Option<&str>,
    ) {
        let now = Utc::now().to_rfc3339();
        let full_name = format!("{owner}/{name}");
        pool.with_conn(|conn| {
            conn.execute(
                "INSERT INTO remote_repositories (
                    id, account_id, platform, remote_id, full_name, name, owner,
                    description, visibility, default_branch, html_url, clone_url,
                    is_favorite, synced_at
                ) VALUES (?1, ?2, ?3, ?1, ?4, ?5, ?6, ?7, ?8, 'main',
                    ?9, ?10, ?11, ?12)",
                params![
                    id,
                    account_id,
                    platform,
                    full_name,
                    name,
                    owner,
                    description,
                    visibility,
                    format!("https://example.com/{full_name}"),
                    format!("https://example.com/{full_name}.git"),
                    i64::from(is_favorite),
                    now,
                ],
            )?;
            Ok(())
        })
        .unwrap();
    }

    fn seed_dataset(pool: &DbPool) {
        seed_account(pool, "acc-gh", "github");
        seed_account(pool, "acc-gl", "gitlab");
        seed_repo(
            pool,
            "r1",
            "acc-gh",
            "github",
            "alice",
            "web-app",
            "public",
            true,
            Some("a frontend project"),
        );
        seed_repo(
            pool,
            "r2",
            "acc-gh",
            "github",
            "alice",
            "api-server",
            "private",
            false,
            Some("backend service"),
        );
        seed_repo(
            pool, "r3", "acc-gh", "github", "bob", "tools", "public", false, None,
        );
        seed_repo(
            pool,
            "r4",
            "acc-gl",
            "gitlab",
            "alice",
            "infra",
            "internal",
            true,
            Some("infra automation"),
        );
        seed_repo(
            pool,
            "r5",
            "acc-gl",
            "gitlab",
            "carol",
            "docs",
            "private",
            false,
            Some("documentation"),
        );
    }

    #[test]
    fn filter_by_account_id() {
        let pool = fresh_pool();
        seed_dataset(&pool);
        let filter = RemoteRepoFilter {
            account_id: Some("acc-gh".to_string()),
            ..Default::default()
        };
        let result = list_remote_repositories(&pool, &filter).unwrap();
        assert_eq!(result.len(), 3);
        assert!(result.iter().all(|r| r.account_id == "acc-gh"));
    }

    #[test]
    fn filter_by_platforms() {
        let pool = fresh_pool();
        seed_dataset(&pool);
        let filter = RemoteRepoFilter {
            platforms: vec!["gitlab".to_string()],
            ..Default::default()
        };
        let result = list_remote_repositories(&pool, &filter).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn filter_by_owner() {
        let pool = fresh_pool();
        seed_dataset(&pool);
        let filter = RemoteRepoFilter {
            owners: vec!["alice".to_string()],
            ..Default::default()
        };
        let result = list_remote_repositories(&pool, &filter).unwrap();
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn filter_by_visibility() {
        let pool = fresh_pool();
        seed_dataset(&pool);
        let filter = RemoteRepoFilter {
            visibilities: vec!["public".to_string()],
            ..Default::default()
        };
        let result = list_remote_repositories(&pool, &filter).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn filter_only_favorite() {
        let pool = fresh_pool();
        seed_dataset(&pool);
        let filter = RemoteRepoFilter {
            only_favorite: true,
            ..Default::default()
        };
        let result = list_remote_repositories(&pool, &filter).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|r| r.is_favorite));
    }

    #[test]
    fn filter_by_search_keyword_matches_name_or_description() {
        let pool = fresh_pool();
        seed_dataset(&pool);

        let by_name = list_remote_repositories(
            &pool,
            &RemoteRepoFilter {
                search: Some("api-server".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(by_name.len(), 1);
        assert_eq!(by_name[0].name, "api-server");

        let by_desc = list_remote_repositories(
            &pool,
            &RemoteRepoFilter {
                search: Some("documentation".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(by_desc.len(), 1);
        assert_eq!(by_desc[0].name, "docs");
    }

    #[test]
    fn filter_combination_account_and_visibility() {
        let pool = fresh_pool();
        seed_dataset(&pool);
        let filter = RemoteRepoFilter {
            account_id: Some("acc-gh".to_string()),
            visibilities: vec!["public".to_string()],
            ..Default::default()
        };
        let result = list_remote_repositories(&pool, &filter).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result
            .iter()
            .all(|r| r.account_id == "acc-gh" && matches!(r.visibility, Visibility::Public)));
    }

    #[test]
    fn toggle_favorite_round_trip() {
        let pool = fresh_pool();
        seed_dataset(&pool);
        let new_state = toggle_favorite(&pool, "r3").unwrap();
        assert!(new_state);
        let toggled_back = toggle_favorite(&pool, "r3").unwrap();
        assert!(!toggled_back);
    }

    #[test]
    fn empty_filter_returns_all() {
        let pool = fresh_pool();
        seed_dataset(&pool);
        let result = list_remote_repositories(&pool, &RemoteRepoFilter::default()).unwrap();
        assert_eq!(result.len(), 5);
    }
}

// =====================================================================
// 本地仓库管理（T070）
// =====================================================================

/// 添加单个本地仓库。
pub async fn add_local_repository(pool: &DbPool, path: &Path) -> Result<LocalRepository> {
    if !path.exists() {
        return Err(GitViewError::PathMissing(path.display().to_string()));
    }
    if !is_git_repository(path) {
        return Err(GitViewError::GitCommand(format!(
            "{} 不是 Git 仓库",
            path.display()
        )));
    }

    let path_str = path.to_string_lossy().to_string();
    let existing: Option<String> = pool.with_conn(|conn| {
        conn.query_row(
            "SELECT id FROM local_repositories WHERE local_path = ?1",
            [&path_str],
            |row| row.get(0),
        )
        .optional()
        .map_err(GitViewError::from)
    })?;

    if let Some(id) = existing {
        return load_local_repository(pool, &id);
    }

    let git_status = git_reader_service::status(path).await?;
    let repo_status = git_reader_service::derive_repository_status(path, &git_status);

    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let status_str = serialize_repo_status(repo_status);
    let branch = git_status.current_branch.clone();
    let remote_url = read_remote_url(path).await.ok();

    let id_clone = id.clone();
    pool.with_conn(move |conn| {
        conn.execute(
            "INSERT INTO local_repositories (
                id, remote_repository_id, local_path, current_branch,
                remote_url, status, last_checked_at, created_at
            ) VALUES (?1, NULL, ?2, ?3, ?4, ?5, ?6, ?6)",
            params![id_clone, path_str, branch, remote_url, status_str, now],
        )
        .map_err(GitViewError::from)?;
        Ok(())
    })?;

    load_local_repository(pool, &id)
}

/// 扫描目录下的所有 Git 仓库并批量添加。
pub async fn scan_local_repositories(
    pool: &DbPool,
    root: &Path,
    max_depth: usize,
) -> Result<ScanResult> {
    if !root.exists() {
        return Err(GitViewError::PathMissing(root.display().to_string()));
    }

    let mut found_paths: Vec<PathBuf> = Vec::new();

    for entry in WalkDir::new(root)
        .max_depth(max_depth)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            name != ".git" && name != "node_modules" && name != "target"
        })
    {
        let Ok(entry) = entry else { continue };
        if entry.file_type().is_dir() && is_git_repository(entry.path()) {
            found_paths.push(entry.path().to_path_buf());
        }
    }

    let mut added = Vec::new();
    for path in &found_paths {
        if let Ok(repo) = add_local_repository(pool, path).await {
            added.push(repo);
        }
    }

    // 清理：扫描父目录之下、但本次未扫到（磁盘已删除）的旧记录。
    // 仅限 root 子树——绝不影响扫描目录之外的仓库（手动添加 / 其他目录扫描来的）。
    let found_set: HashSet<String> = found_paths
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    let mut removed = 0usize;
    for repo in list_local_repositories(pool)? {
        // starts_with 按路径组件比较，避免 `/a/foo` 误判为 `/a/foo-bar` 的子路径
        if Path::new(&repo.local_path).starts_with(root) && !found_set.contains(&repo.local_path) {
            remove_local_repository(pool, &repo.id)?;
            removed += 1;
        }
    }

    Ok(ScanResult { added, removed })
}

/// 从列表移除本地仓库（仅删除数据库记录，不删除磁盘文件）。
pub fn remove_local_repository(pool: &DbPool, id: &str) -> Result<()> {
    let id = id.to_string();
    pool.with_conn(move |conn| {
        conn.execute("DELETE FROM local_repositories WHERE id = ?1", [&id])
            .map_err(GitViewError::from)?;
        Ok(())
    })
}

/// 列出所有本地仓库。
pub fn list_local_repositories(pool: &DbPool) -> Result<Vec<LocalRepository>> {
    pool.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, remote_repository_id, local_path, current_branch,
                    remote_url, status, last_checked_at, created_at
             FROM local_repositories
             ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], row_to_local_repo)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    })
}

/// 刷新单个仓库状态。
pub async fn refresh_local_repository_status(pool: &DbPool, id: &str) -> Result<LocalRepository> {
    let repo = load_local_repository(pool, id)?;
    let path = Path::new(&repo.local_path);

    let (status_str, branch) = if path.exists() {
        let git_status = git_reader_service::status(path).await?;
        let derived = git_reader_service::derive_repository_status(path, &git_status);
        (serialize_repo_status(derived), git_status.current_branch)
    } else {
        ("unknown".to_string(), None)
    };

    let now = Utc::now().to_rfc3339();
    let id_owned = id.to_string();
    pool.with_conn(move |conn| {
        conn.execute(
            "UPDATE local_repositories
             SET status = ?1, current_branch = ?2, last_checked_at = ?3
             WHERE id = ?4",
            params![status_str, branch, now, id_owned],
        )
        .map_err(GitViewError::from)?;
        Ok(())
    })?;

    load_local_repository(pool, id)
}

/// 刷新所有本地仓库状态。
pub async fn refresh_all_local_repository_status(pool: &DbPool) -> Result<Vec<LocalRepository>> {
    let repos = list_local_repositories(pool)?;
    for repo in &repos {
        let _ = refresh_local_repository_status(pool, &repo.id).await;
    }
    list_local_repositories(pool)
}

// =====================================================================
// 批量 Fetch（T071）
// =====================================================================

/// 批量 Fetch 结果摘要。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchFetchSummary {
    pub total: usize,
    pub success: usize,
    pub failed: usize,
    pub failures: Vec<FetchFailure>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchFailure {
    pub repo_id: String,
    pub repo_name: String,
    pub error: String,
}

/// 并行 Fetch 多个仓库。
pub async fn batch_fetch(pool: &DbPool, ids: Vec<String>) -> Result<BatchFetchSummary> {
    let repos: Vec<LocalRepository> = ids
        .iter()
        .filter_map(|id| load_local_repository(pool, id).ok())
        .collect();

    let total = repos.len();
    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(4));
    let mut handles = Vec::new();

    for repo in repos {
        let sem = sem.clone();
        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await;
            let path = PathBuf::from(&repo.local_path);
            let result = crate::utils::process::run_command(
                "git",
                &["fetch", "--all", "--prune"],
                &[],
                Some(&path),
            )
            .await;
            (repo.id, repo.local_path, result)
        }));
    }

    let mut success = 0;
    let mut failures = Vec::new();

    for handle in handles {
        if let Ok((id, path, result)) = handle.await {
            match result {
                Ok(output) if output.status.success() => success += 1,
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    failures.push(FetchFailure {
                        repo_id: id,
                        repo_name: path,
                        error: crate::utils::redact::redact_token(&stderr),
                    });
                }
                Err(e) => {
                    failures.push(FetchFailure {
                        repo_id: id,
                        repo_name: path,
                        error: e.to_string(),
                    });
                }
            }
        }
    }

    Ok(BatchFetchSummary {
        total,
        success,
        failed: failures.len(),
        failures,
    })
}

// =====================================================================
// 本地仓库辅助
// =====================================================================

/// 按 id 加载本地仓库记录。
///
/// 供 commands/git.rs 等需要先解析 repo path 的调用方使用。
pub fn load_local_repository(pool: &DbPool, id: &str) -> Result<LocalRepository> {
    let id = id.to_string();
    pool.with_conn(move |conn| {
        conn.query_row(
            "SELECT id, remote_repository_id, local_path, current_branch,
                    remote_url, status, last_checked_at, created_at
             FROM local_repositories WHERE id = ?1",
            [&id],
            row_to_local_repo,
        )
        .map_err(GitViewError::from)
    })
}

async fn read_remote_url(repo_path: &Path) -> Result<String> {
    let output = crate::utils::process::run_command(
        "git",
        &["remote", "get-url", "origin"],
        &[],
        Some(repo_path),
    )
    .await?;
    if !output.status.success() {
        return Err(GitViewError::NotFound("无 origin 远端".to_string()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn serialize_repo_status(s: RepositoryStatus) -> String {
    match s {
        RepositoryStatus::Clean => "clean",
        RepositoryStatus::Dirty => "dirty",
        RepositoryStatus::Ahead => "ahead",
        RepositoryStatus::Behind => "behind",
        RepositoryStatus::Diverged => "diverged",
        RepositoryStatus::Unknown => "unknown",
    }
    .to_string()
}

fn deserialize_repo_status(s: &str) -> RepositoryStatus {
    match s {
        "clean" => RepositoryStatus::Clean,
        "dirty" => RepositoryStatus::Dirty,
        "ahead" => RepositoryStatus::Ahead,
        "behind" => RepositoryStatus::Behind,
        "diverged" => RepositoryStatus::Diverged,
        _ => RepositoryStatus::Unknown,
    }
}

fn row_to_local_repo(row: &rusqlite::Row<'_>) -> rusqlite::Result<LocalRepository> {
    use chrono::DateTime;

    let status_str: String = row.get("status")?;
    let checked_str: String = row.get("last_checked_at")?;
    let created_str: String = row.get("created_at")?;

    let parse_dt = |s: &str| -> rusqlite::Result<DateTime<Utc>> {
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

    Ok(LocalRepository {
        id: row.get("id")?,
        remote_repository_id: row.get("remote_repository_id")?,
        local_path: row.get("local_path")?,
        current_branch: row.get("current_branch")?,
        remote_url: row.get("remote_url")?,
        status: deserialize_repo_status(&status_str),
        last_checked_at: parse_dt(&checked_str)?,
        created_at: parse_dt(&created_str)?,
    })
}
