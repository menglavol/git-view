//! GitView 领域模型模块。
//!
//! 与前端 `src/types/` 一一对应，作为 service / command 层与前端的契约。
//! 所有结构体派生 `Debug, Clone, Serialize, Deserialize`；
//! 枚举使用 `#[serde(rename_all = "snake_case")]` 与前端字面量联合类型对齐；
//! 时间字段使用 `chrono::DateTime<Utc>` 序列化为 ISO 8601。
//!
//! 安全约束（宪法 Principle III）：
//!   - Account 等模型禁止包含 token 明文字段；只保留 `token_key` 引用
//!   - 凭据查找一律通过 `services::credential_service` 经 keyring 解密

pub mod account;
pub mod clone_task;
pub mod git;
pub mod operation_log;
pub mod repository;
pub mod settings;
