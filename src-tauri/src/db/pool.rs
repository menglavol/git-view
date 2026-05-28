//! SQLite 连接池模块。
//!
//! 提供线程安全的数据库连接管理：
//!   - `DbPool` 结构体封装 `Arc<Mutex<Connection>>`
//!   - `with_conn` 闭包式访问，自动获取/释放锁
//!   - 首次连接时自动创建数据库文件与目录
//!   - 启用 WAL 模式与外键约束
//!
//! V1 采用简单互斥锁满足单进程访问需求，不引入 r2d2 连接池。
//! 后续版本若需多连接并发读可升级为 r2d2 或 deadpool。

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::errors::{GitViewError, Result};

/// SQLite 连接池。
///
/// 内部使用 `Arc<Mutex<Connection>>` 实现线程安全的单连接访问。
/// 通过 `with_conn` 方法以闭包方式获取连接引用，避免锁泄漏。
#[derive(Clone)]
pub struct DbPool {
    /// 互斥锁保护的 SQLite 连接
    inner: Arc<Mutex<Connection>>,
    /// 数据库文件路径（用于日志与诊断）
    db_path: PathBuf,
}

impl DbPool {
    /// 创建新的连接池实例。
    ///
    /// 若数据库文件或其父目录不存在，将自动创建。
    /// 连接建立后立即执行 PRAGMA 配置（WAL、外键、application_id）。
    ///
    /// # Arguments
    ///
    /// * `db_path` - 数据库文件的完整路径
    ///
    /// # Errors
    ///
    /// 当路径创建失败或 SQLite 连接打开失败时返回 `Database` 错误。
    pub fn new(db_path: &Path) -> Result<Self> {
        // 确保父目录存在
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                GitViewError::Database(format!("无法创建数据库目录 {}: {}", parent.display(), e))
            })?;
        }

        // 打开或创建数据库文件
        let conn = Connection::open(db_path)?;

        // 配置 PRAGMA：WAL 模式提升并发读性能
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;
             PRAGMA application_id = 0x47565757;",
            // application_id = "GVWW" 的十六进制，防止误打开非本应用数据库
        )?;

        Ok(Self {
            inner: Arc::new(Mutex::new(conn)),
            db_path: db_path.to_path_buf(),
        })
    }

    /// 获取默认数据库路径。
    ///
    /// 路径规则：
    ///   - macOS: `~/Library/Application Support/gitview/gitview.db`
    ///   - Windows: `%LOCALAPPDATA%/gitview/gitview.db`
    ///   - Linux: `~/.local/share/gitview/gitview.db`
    #[must_use]
    pub fn default_path() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("gitview")
            .join("gitview.db")
    }

    /// 以闭包方式访问数据库连接。
    ///
    /// 自动获取互斥锁，闭包执行完毕后释放。
    /// 若锁被 poison（持有线程 panic），返回内部错误。
    ///
    /// # Arguments
    ///
    /// * `f` - 接收 `&Connection` 引用的闭包
    ///
    /// # Returns
    ///
    /// 闭包的返回值，包装在 `Result` 中。
    pub fn with_conn<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&Connection) -> Result<R>,
    {
        let conn = self.inner.lock().map_err(|e| {
            GitViewError::Database(format!("数据库锁获取失败（Mutex poisoned）: {e}"))
        })?;
        f(&conn)
    }

    /// 获取数据库文件路径（用于日志输出）。
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.db_path
    }
}

// =====================================================================
// 单元测试
// =====================================================================

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    /// 测试：创建临时数据库并执行简单查询
    #[test]
    fn test_create_and_query() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let pool = DbPool::new(tmp.path()).unwrap();

        let result = pool.with_conn(|conn| {
            conn.execute(
                "CREATE TABLE test_table (id INTEGER PRIMARY KEY, name TEXT)",
                [],
            )
            .map_err(|e| GitViewError::Database(e.to_string()))?;

            conn.execute("INSERT INTO test_table (name) VALUES (?1)", ["hello"])
                .map_err(|e| GitViewError::Database(e.to_string()))?;

            let name: String = conn
                .query_row("SELECT name FROM test_table WHERE id = 1", [], |row| {
                    row.get(0)
                })
                .map_err(|e| GitViewError::Database(e.to_string()))?;

            Ok(name)
        });

        assert_eq!(result.unwrap(), "hello");
    }

    /// 测试：WAL 模式已启用
    #[test]
    fn test_wal_mode_enabled() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let pool = DbPool::new(tmp.path()).unwrap();

        let mode = pool.with_conn(|conn| {
            let mode: String = conn
                .query_row("PRAGMA journal_mode", [], |row| row.get(0))
                .map_err(|e| GitViewError::Database(e.to_string()))?;
            Ok(mode)
        });

        assert_eq!(mode.unwrap(), "wal");
    }
}
