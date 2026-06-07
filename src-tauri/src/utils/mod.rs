//! GitView 工具模块。
//!
//! 提供跨 service 复用的通用辅助函数：
//!   - `data_dir` — 数据目录指针文件读写（可配置数据目录的固定位置指针）
//!   - `path`    — 路径规范化、目录创建、Git 仓库检测
//!   - `process` — 异步子进程执行封装（自动注入环境变量）
//!   - `redact`  — Token / 凭据脱敏（安全门禁）
//!   - `time`    — ISO 8601 时间格式化与解析

pub mod data_dir;
pub mod path;
pub mod process;
pub mod redact;
pub mod time;
