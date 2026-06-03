//! 操作日志相关 Tauri commands（US6 / T096）。
//!
//! 本模块是「操作日志与诊断」用户故事的前端入口，薄包装 `log_service`：
//!   - `list_operation_logs`       — 按筛选条件分页查询日志
//!   - `get_operation_log_detail`  — 查询单条日志详情
//!   - `clear_old_operation_logs`  — 清理旧日志（破坏性，前端须二次确认）
//!
//! 安全约束：日志在 `log_service` 写入时已统一脱敏；清理操作属宪法
//! Principle III 删除范畴，前端 MUST 通过 `ConfirmDangerDialog` 二次确认。

#![allow(clippy::needless_pass_by_value)]

use tauri::State;

use crate::errors::Result;
use crate::models::operation_log::{LogFilter, OperationLog};
use crate::services::log_service;
use crate::AppState;

/// 按筛选条件分页查询操作日志（按发生时间倒序，含中文错误翻译）。
#[tauri::command]
pub fn list_operation_logs(
    state: State<'_, AppState>,
    filter: LogFilter,
) -> Result<Vec<OperationLog>> {
    log_service::list_operations(&state.db, &filter)
}

/// 查询单条操作日志详情（含中文错误翻译）。
#[tauri::command]
pub fn get_operation_log_detail(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<OperationLog>> {
    log_service::get_operation_detail(&state.db, &id)
}

/// 清理操作日志。
///
/// `before_days = None` 清空全部；`Some(n)` 删除 n 天前的日志。
/// **破坏性操作**：前端调用前 MUST 经 `ConfirmDangerDialog` 二次确认
/// （宪法 Principle III）。返回删除的行数。
#[tauri::command]
pub fn clear_old_operation_logs(
    state: State<'_, AppState>,
    before_days: Option<u32>,
) -> Result<usize> {
    log_service::clear_operations_older_than(&state.db, before_days)
}
