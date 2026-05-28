//! Clone 任务相关 Tauri commands。

#![allow(clippy::needless_pass_by_value)]

use tauri::{AppHandle, State};

use crate::errors::Result;
use crate::models::clone_task::CloneTask;
use crate::services::clone_task_service::{self, CreateCloneTasksPayload};
use crate::AppState;

/// 创建一批 clone 任务（status = pending），不立即执行。
#[tauri::command]
pub fn create_clone_tasks(
    state: State<'_, AppState>,
    payload: CreateCloneTasksPayload,
) -> Result<Vec<CloneTask>> {
    clone_task_service::create_clone_tasks(&state.db, &payload)
}

/// 启动一批 pending / failed 任务。
///
/// 返回类型保留 `Result<()>` 与其他 command 对齐——前端 `invokeCmd` 统一按
/// Result 形态解包，移除 Result 会让前端 try/catch 分支断裂。
#[tauri::command]
#[allow(clippy::unnecessary_wraps)]
pub fn start_clone_tasks(
    app: AppHandle,
    state: State<'_, AppState>,
    task_ids: Vec<String>,
    auto_add_to_local: bool,
) -> Result<()> {
    clone_task_service::start_clone_tasks(
        app,
        state.db.clone(),
        state.clone_manager.clone(),
        task_ids,
        auto_add_to_local,
    );
    Ok(())
}

#[tauri::command]
pub fn list_clone_tasks(state: State<'_, AppState>) -> Result<Vec<CloneTask>> {
    clone_task_service::list_clone_tasks(&state.db)
}

#[tauri::command]
pub fn cancel_clone_task(state: State<'_, AppState>, task_id: String) -> Result<()> {
    clone_task_service::cancel_clone_task(&state.clone_manager, &task_id)
}

#[tauri::command]
pub fn retry_clone_task(
    app: AppHandle,
    state: State<'_, AppState>,
    task_id: String,
    auto_add_to_local: bool,
) -> Result<()> {
    clone_task_service::retry_clone_task(
        app,
        &state.db,
        state.clone_manager.clone(),
        &task_id,
        auto_add_to_local,
    )
}

#[tauri::command]
pub fn clear_finished_clone_tasks(state: State<'_, AppState>) -> Result<usize> {
    clone_task_service::clear_finished_clone_tasks(&state.db)
}
