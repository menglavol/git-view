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

use std::time::Instant;

use tauri::State;

use crate::db::pool::DbPool;
use crate::errors::{GitViewError, Result};
use crate::models::account::{Account, AccountUpdate, AddAccountPayload, TestConnectionPayload};
use crate::models::operation_log::{OperationStatus, OperationType};
use crate::services::provider::UserProfile;
use crate::services::{account_service, log_service};
use crate::AppState;

/// 记录一次账号相关操作日志（US6）。
///
/// 写入失败不影响主流程（log_service 内部已记 warn）。
/// `UserCancelled` → Cancelled，其余错误 → Failed，无错误 → Success。
fn log_account(
    db: &DbPool,
    op: OperationType,
    target: &str,
    start: Instant,
    error: Option<&GitViewError>,
    output: Option<&str>,
) {
    // 操作耗时远不会溢出 u64，截断分支不可达
    #[allow(clippy::cast_possible_truncation)]
    let duration_ms = start.elapsed().as_millis() as u64;
    // 错误类型决定日志状态：
    //   - 无错误 → Success
    //   - 用户主动取消（UserCancelled）→ Cancelled，避免在日志里误显示为红色失败
    //   - 其余错误 → Failed，并把（已脱敏的）错误文案一并写入
    let (status, msg) = match error {
        None => (OperationStatus::Success, None),
        Some(GitViewError::UserCancelled) => (OperationStatus::Cancelled, None),
        Some(e) => (OperationStatus::Failed, Some(e.to_string())),
    };
    // 以 `let _ =` 吞掉写日志的错误：日志是旁路能力，绝不能反向阻断账号主操作
    let _ = log_service::record_operation(
        db,
        op,
        target,
        status,
        None,
        output,
        msg.as_deref(),
        duration_ms,
    );
}

/// 添加账号（含连接测试、数据库事务、keyring 保存）。
///
/// service 内部按「测试连接 → 事务插入 accounts → keyring 落 token」顺序执行，
/// 任一步失败则整体回滚，杜绝「有账号无凭据」或「有凭据无账号」的脏状态。
#[tauri::command]
pub async fn add_account(
    state: State<'_, AppState>,
    payload: AddAccountPayload,
) -> Result<Account> {
    let start = Instant::now();
    let result = account_service::add_account(&state.db, payload).await;
    // 成功时以用户名为 target，失败时退化为固定描述（payload 已被 service 消费，无法回取）
    let target = result
        .as_ref()
        .map_or_else(|_| "添加账号".to_string(), |a| a.username.clone());
    // 无论成败都落一条日志，便于用户在「操作日志」页回溯账号变更
    log_account(
        &state.db,
        OperationType::AddAccount,
        &target,
        start,
        result.as_ref().err(),
        None,
    );
    result
}

/// 测试账号连接（不写入数据库 / keyring）。
///
/// 用于「添加账号」表单的实时校验：仅探测 token 能否换到用户信息，无任何副作用。
#[tauri::command]
pub async fn test_account_connection(
    state: State<'_, AppState>,
    payload: TestConnectionPayload,
) -> Result<UserProfile> {
    let start = Instant::now();
    let result = account_service::test_account_connection(payload).await;
    // 成功时以探测到的用户名为 target，失败时退化为固定描述
    let target = result
        .as_ref()
        .map_or_else(|_| "测试连接".to_string(), |u| u.username.clone());
    log_account(
        &state.db,
        OperationType::TestConnection,
        &target,
        start,
        result.as_ref().err(),
        None,
    );
    result
}

/// 列出所有账号（默认账号优先）。
///
/// 纯读取、无副作用，因此不记操作日志（避免日志被高频读操作淹没）。
#[tauri::command]
pub fn list_accounts(state: State<'_, AppState>) -> Result<Vec<Account>> {
    account_service::list_accounts(&state.db)
}

/// 更新账号的可选字段（display_name / remark / enabled 等）。
///
/// `enabled = false` 对应 FR-009 账号禁用：禁用后该账号不再参与同步与默认筛选。
#[tauri::command]
pub fn update_account(
    state: State<'_, AppState>,
    id: String,
    fields: AccountUpdate,
) -> Result<Account> {
    account_service::update_account(&state.db, &id, fields)
}

/// 删除账号（先清理 keyring 再删数据库行）。
///
/// service 内部保证：被删的若是默认账号，则把默认标记转移到剩余 enabled 账号中最早的一个。
#[tauri::command]
pub fn delete_account(state: State<'_, AppState>, id: String) -> Result<()> {
    let start = Instant::now();
    let result = account_service::delete_account(&state.db, &id);
    // 删除属敏感操作，记一条日志留痕（前端已通过 ConfirmDangerDialog 二次确认）
    log_account(
        &state.db,
        OperationType::DeleteAccount,
        &id,
        start,
        result.as_ref().err(),
        None,
    );
    result
}

/// 把指定账号设为默认账号。
///
/// service 内部原子切换（先全部置 0 再把目标置 1）；目标账号必须为 enabled。
#[tauri::command]
pub fn set_default_account(state: State<'_, AppState>, id: String) -> Result<()> {
    account_service::set_default_account(&state.db, &id)
}

/// 同步账号下的远程仓库。
///
/// 通过 `account_service_state` 维护「正在同步的账号集合」，实现同账号互斥、
/// 不同账号可并行（对应 spec Edge Case「多账号同时同步」）。
#[tauri::command]
pub async fn sync_account_repositories(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<usize> {
    let start = Instant::now();
    let result = account_service::sync_account_repositories(
        &state.account_service_state,
        &state.db,
        &account_id,
    )
    .await;
    // 成功时把同步到的仓库数量写进日志输出，便于回看每次同步规模
    let output = result.as_ref().ok().map(|n| format!("同步 {n} 个仓库"));
    log_account(
        &state.db,
        OperationType::SyncRepos,
        &account_id,
        start,
        result.as_ref().err(),
        output.as_deref(),
    );
    result
}
