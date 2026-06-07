//! GitView Tauri commands 模块。
//!
//! 各业务 command 文件按域分组：
//!   - `accounts`             — 账号 CRUD / 测试连接 / 默认账号
//!   - `remote_repositories`  — 远程仓库查询 / 同步 / 收藏
//!   - `clone_tasks`          — 批量 Clone 任务调度
//!   - `local_repositories`   — 本地仓库集中管理（US4）
//!   - `git`                  — 单仓库 Git 工作流（US5：15 个命令）
//!   - `logs`                 — 操作日志与诊断（US6：3 个命令）
//!   - `settings`             — 设置与默认目录管理（US7：4 个命令）

pub mod accounts;
pub mod clone_tasks;
pub mod data_dir;
pub mod git;
pub mod local_repositories;
pub mod logs;
pub mod remote_repositories;
pub mod settings;
