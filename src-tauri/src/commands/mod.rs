//! GitView Tauri commands 模块。
//!
//! 各业务 command 文件按域分组：
//!   - `accounts`             — 账号 CRUD / 测试连接 / 默认账号
//!   - `remote_repositories`  — 远程仓库查询 / 同步 / 收藏
//!   - `clone_tasks`          — 批量 Clone 任务调度
//!   - `local_repositories`   — 本地仓库集中管理（US4）

pub mod accounts;
pub mod clone_tasks;
pub mod local_repositories;
pub mod remote_repositories;
