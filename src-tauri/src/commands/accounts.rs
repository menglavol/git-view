//! 账号相关 Tauri commands。
//!
//! 本模块是前端与 `account_service` 之间的薄薄一层胶水：
//!   - 反序列化 payload
//!   - 调用 service
//!   - 处理 service 错误（自动序列化为 `{ code, detail }`）
//!
//! 安全约束：
//!   - payload 中包含 token 明文，处理完后立即由 service 写入 keyring 后释放
//!   - 返回值绝不包含 token 字段（model 层已保证）
//!
//! Lint 注记：Tauri command 签名要求 `State<T>`、Payload struct 等按值接收，
//! 因此模块级抑制 `needless_pass_by_value`。

#![allow(clippy::needless_pass_by_value)]

use tauri::State;

use crate::errors::Result;
use crate::models::account::{Account, AccountUpdate, AddAccountPayload, TestConnectionPayload};
use crate::services::account_service;
use crate::services::provider::UserProfile;
use crate::AppState;

/// 添加账号（含连接测试、数据库事务、keyring 保存）。
#[tauri::command]
pub async fn add_account(
    state: State<'_, AppState>,
    payload: AddAccountPayload,
) -> Result<Account> {
    account_service::add_account(&state.db, payload).await
}

/// 测试账号连接（不写入数据库 / keyring）。
#[tauri::command]
pub async fn test_account_connection(payload: TestConnectionPayload) -> Result<UserProfile> {
    account_service::test_account_connection(payload).await
}

/// 列出所有账号（默认账号优先）。
#[tauri::command]
pub fn list_accounts(state: State<'_, AppState>) -> Result<Vec<Account>> {
    account_service::list_accounts(&state.db)
}

/// 更新账号的可选字段。
#[tauri::command]
pub fn update_account(
    state: State<'_, AppState>,
    id: String,
    fields: AccountUpdate,
) -> Result<Account> {
    account_service::update_account(&state.db, &id, fields)
}

/// 删除账号（先清理 keyring 再删数据库行）。
#[tauri::command]
pub fn delete_account(state: State<'_, AppState>, id: String) -> Result<()> {
    account_service::delete_account(&state.db, &id)
}

/// 把指定账号设为默认账号。
#[tauri::command]
pub fn set_default_account(state: State<'_, AppState>, id: String) -> Result<()> {
    account_service::set_default_account(&state.db, &id)
}

/// 同步账号下的远程仓库。
#[tauri::command]
pub async fn sync_account_repositories(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<usize> {
    account_service::sync_account_repositories(&state.account_service_state, &state.db, &account_id)
        .await
}
