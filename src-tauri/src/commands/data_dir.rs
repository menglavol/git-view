//! 数据目录相关 Tauri commands（US7）。
//!
//! 薄包装 `data_dir_service`：读取当前 / 旧数据目录、迁移、删除旧目录，以及触发
//! 应用重启（迁移后需重启使新目录生效）。数据目录位置存于固定指针文件而非 DB
//! （鸡生蛋，见 `utils::data_dir`），故这些命令不经过 settings 设置组。

#![allow(clippy::needless_pass_by_value)]

use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::State;

use crate::errors::Result;
use crate::services::data_dir_service::{self, MigrateResult, OldDataDir};
use crate::utils::path::app_data_dir;
use crate::AppState;

/// 当前数据目录信息（当前生效目录 + 可选的待删旧目录路径）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataDirInfo {
    /// 当前生效的数据目录绝对路径
    pub current: String,
    /// 迁移后保留的旧目录路径（无则为 None）
    pub previous: Option<String>,
}

/// 读取当前数据目录信息（当前路径 + 是否存在待删旧目录）。
///
/// 旧目录仅取路径并校验仍存在；占用统计较重，按需走 `get_old_data_dir`。
#[tauri::command]
#[must_use]
pub fn get_data_dir() -> DataDirInfo {
    let current = app_data_dir().to_string_lossy().into_owned();
    // 读指针拿 previousDir，并过滤掉已被用户从文件系统删掉的路径
    let previous = crate::utils::data_dir::read_pointer()
        .and_then(|p| p.previous_dir)
        .filter(|p| Path::new(p).is_dir());
    DataDirInfo { current, previous }
}

/// 迁移数据目录：把当前 DB + 日志复制到 `new_dir`，更新指针，旧目录保留。
///
/// 迁移后需重启应用才生效（前端引导）。校验 / 复制失败时返回错误，指针不动。
#[tauri::command]
pub fn migrate_data_dir(state: State<'_, AppState>, new_dir: String) -> Result<MigrateResult> {
    data_dir_service::migrate(&state.db, &PathBuf::from(new_dir))
}

/// 读取旧数据目录占用（路径 + 大小 + 文件数）；无旧目录时返回 `None`。
#[tauri::command]
#[must_use]
pub fn get_old_data_dir() -> Option<OldDataDir> {
    data_dir_service::old_data_dir()
}

/// 删除旧数据目录并清空指针的 previousDir 字段。
#[tauri::command]
pub fn delete_old_data_dir() -> Result<()> {
    data_dir_service::delete_old_data_dir()
}

/// 重启应用（迁移后使新数据目录生效）。
///
/// `AppHandle::restart` 不返回（`-> !`）；进程随即重启，前端不会收到返回值。
#[tauri::command]
pub fn restart_app(app: tauri::AppHandle) {
    app.restart();
}
