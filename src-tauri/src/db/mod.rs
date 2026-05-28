//! GitView 数据库模块。
//!
//! 提供 SQLite 连接池管理与 schema 迁移能力：
//!   - `pool`       — 连接池（Arc<Mutex<Connection>>）与闭包式访问
//!   - `migrations` — 版本化迁移管理（内嵌 SQL 文件顺序执行）
//!
//! 数据库文件位置：`<data_local_dir>/gitview/gitview.db`
//! 启用 WAL 模式以提升并发读性能，启用外键约束保证引用完整性。

pub mod migrations;
pub mod pool;
