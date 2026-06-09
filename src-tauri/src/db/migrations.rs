//! SQLite Schema 迁移管理。
//!
//! 通过 `schema_migrations(version, applied_at)` 表追踪已应用版本，
//! 每个版本对应一个 `migrations/NNN_*.sql` 文件，按 version 升序执行。
//!
//! 设计原则：
//!   - 迁移文件在编译期通过 `include_str!` 嵌入二进制，避免运行时文件依赖
//!   - 每个迁移在事务中执行，部分语句失败时回滚整个迁移
//!   - 已应用版本不重复执行，保证幂等

use rusqlite::Connection;

use crate::db::pool::DbPool;
use crate::errors::{GitViewError, Result};

/// 单个迁移描述。
///
/// `version` 严格递增；`sql` 在编译期嵌入。
struct Migration {
    version: i64,
    name: &'static str,
    sql: &'static str,
}

/// 全部迁移定义（按 version 升序）。
///
/// 新增迁移时：在末尾追加条目即可；既不修改也不删除已发布的历史条目。
const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "001_init",
        sql: include_str!("migrations/001_init.sql"),
    },
    Migration {
        version: 2,
        name: "002_extend_account_and_gitlab",
        sql: include_str!("migrations/002_extend_account_and_gitlab.sql"),
    },
    Migration {
        version: 3,
        name: "003_extend_operation_logs",
        sql: include_str!("migrations/003_extend_operation_logs.sql"),
    },
    Migration {
        version: 4,
        name: "004_add_account_clone_protocol",
        sql: include_str!("migrations/004_add_account_clone_protocol.sql"),
    },
];

/// 在指定连接池上运行所有未应用的迁移。
///
/// 流程：
///   1. 创建 `schema_migrations` 表（首次启动时）
///   2. 读取已应用版本集合
///   3. 逐个执行未应用版本的迁移 SQL（事务内）
///   4. 在 `schema_migrations` 中记录新应用版本
///
/// # Returns
///
/// 实际应用的迁移数量；若全部迁移已应用则返回 0。
pub fn run_pending_migrations(pool: &DbPool) -> Result<usize> {
    pool.with_conn(|conn| {
        ensure_migrations_table(conn)?;
        let applied = load_applied_versions(conn)?;

        let mut applied_count = 0;
        for migration in MIGRATIONS {
            if applied.contains(&migration.version) {
                continue;
            }
            apply_migration(conn, migration)?;
            applied_count += 1;
        }

        Ok(applied_count)
    })
}

/// 创建迁移版本追踪表（若不存在）。
fn ensure_migrations_table(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
             version    INTEGER PRIMARY KEY,
             name       TEXT NOT NULL,
             applied_at TEXT NOT NULL
         )",
        [],
    )?;
    Ok(())
}

/// 读取所有已应用迁移的版本号。
fn load_applied_versions(conn: &Connection) -> Result<Vec<i64>> {
    let mut stmt = conn.prepare("SELECT version FROM schema_migrations")?;
    let rows = stmt.query_map([], |row| row.get::<_, i64>(0))?;

    let mut versions = Vec::new();
    for row in rows {
        versions.push(row?);
    }
    Ok(versions)
}

/// 在事务中应用单个迁移并记录版本。
fn apply_migration(conn: &Connection, migration: &Migration) -> Result<()> {
    // 注意：rusqlite 的 execute_batch 已隐式处理多语句；
    // 这里手动开事务以保证 SQL + 版本记录的原子性。
    conn.execute_batch("BEGIN;")?;

    // 真正执行迁移 SQL —— 失败时回滚
    if let Err(e) = conn.execute_batch(migration.sql) {
        let _ = conn.execute_batch("ROLLBACK;");
        return Err(GitViewError::Database(format!(
            "迁移 {} 执行失败：{}",
            migration.name, e
        )));
    }

    // 记录版本
    let applied_at = crate::utils::time::now_iso8601();
    if let Err(e) = conn.execute(
        "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
        rusqlite::params![migration.version, migration.name, applied_at],
    ) {
        let _ = conn.execute_batch("ROLLBACK;");
        return Err(GitViewError::Database(format!(
            "记录迁移版本 {} 失败：{}",
            migration.version, e
        )));
    }

    conn.execute_batch("COMMIT;")?;
    Ok(())
}

// =====================================================================
// 单元测试
// =====================================================================
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    /// 首次运行应用所有迁移
    #[test]
    fn first_run_applies_all() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let pool = DbPool::new(tmp.path()).unwrap();
        let count = run_pending_migrations(&pool).unwrap();
        assert_eq!(count, MIGRATIONS.len());
    }

    /// 二次运行跳过已应用迁移
    #[test]
    fn second_run_is_idempotent() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let pool = DbPool::new(tmp.path()).unwrap();
        let _ = run_pending_migrations(&pool).unwrap();
        let count = run_pending_migrations(&pool).unwrap();
        assert_eq!(count, 0);
    }

    /// 迁移后所有业务表均存在
    #[test]
    fn business_tables_created() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let pool = DbPool::new(tmp.path()).unwrap();
        run_pending_migrations(&pool).unwrap();

        let expected = [
            "accounts",
            "gitlab_instance_configs",
            "remote_repositories",
            "local_repositories",
            "clone_tasks",
            "operation_logs",
            "settings",
            "schema_migrations",
        ];

        pool.with_conn(|conn| {
            for table in expected {
                let exists: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                        rusqlite::params![table],
                        |row| row.get(0),
                    )
                    .map_err(|e| GitViewError::Database(e.to_string()))?;
                assert_eq!(exists, 1, "表 {table} 应被创建");
            }
            Ok(())
        })
        .unwrap();
    }

    /// 外键约束已启用（INSERT 无效外键时应失败）
    #[test]
    fn foreign_keys_enforced() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let pool = DbPool::new(tmp.path()).unwrap();
        run_pending_migrations(&pool).unwrap();

        let result = pool.with_conn(|conn| {
            // 尝试插入 account_id 不存在的远程仓库 —— 应触发外键错误
            conn.execute(
                "INSERT INTO remote_repositories
                 (id, account_id, platform, remote_id, full_name, name, owner,
                  visibility, default_branch, html_url, clone_url, synced_at)
                 VALUES ('r1','nonexistent','github','100','foo/bar','bar','foo',
                         'public','main','https://x','https://x','2025-01-01T00:00:00+00:00')",
                [],
            )
            .map_err(|e| GitViewError::Database(e.to_string()))
        });

        assert!(result.is_err(), "外键约束应阻止悬空 account_id");
    }

    /// migration 003 后 operation_logs 应包含 command 与 output 两列
    #[test]
    fn operation_logs_has_detail_columns() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let pool = DbPool::new(tmp.path()).unwrap();
        run_pending_migrations(&pool).unwrap();

        pool.with_conn(|conn| {
            // PRAGMA table_info 第 2 列（索引 1）为列名
            let mut stmt = conn
                .prepare("PRAGMA table_info(operation_logs)")
                .map_err(|e| GitViewError::Database(e.to_string()))?;
            let cols: Vec<String> = stmt
                .query_map([], |row| row.get::<_, String>(1))
                .map_err(|e| GitViewError::Database(e.to_string()))?
                .filter_map(std::result::Result::ok)
                .collect();
            assert!(
                cols.contains(&"command".to_string()),
                "operation_logs 应含 command 列"
            );
            assert!(
                cols.contains(&"output".to_string()),
                "operation_logs 应含 output 列"
            );
            Ok(())
        })
        .unwrap();
    }

    /// migration 004 后 accounts 应包含 default_clone_protocol 列
    #[test]
    fn accounts_has_clone_protocol_column() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let pool = DbPool::new(tmp.path()).unwrap();
        run_pending_migrations(&pool).unwrap();

        pool.with_conn(|conn| {
            // PRAGMA table_info 第 2 列（索引 1）为列名
            let mut stmt = conn
                .prepare("PRAGMA table_info(accounts)")
                .map_err(|e| GitViewError::Database(e.to_string()))?;
            let cols: Vec<String> = stmt
                .query_map([], |row| row.get::<_, String>(1))
                .map_err(|e| GitViewError::Database(e.to_string()))?
                .filter_map(std::result::Result::ok)
                .collect();
            assert!(
                cols.contains(&"default_clone_protocol".to_string()),
                "accounts 应含 default_clone_protocol 列"
            );
            Ok(())
        })
        .unwrap();
    }
}
