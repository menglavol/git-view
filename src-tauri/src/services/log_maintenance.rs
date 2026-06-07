//! 日志文件维护：统计占用与清理历史日志。
//!
//! 日志由 tracing-appender 按天滚动写入 `utils::path::log_dir()`，
//! 文件名形如 `gitview.log.YYYY-MM-DD`。本模块给设置页提供两项能力：
//!   - 统计日志目录占用（路径 + 总字节 + 文件数）
//!   - 清理历史日志（保留当天，删除更早的滚动文件）

use chrono::Local;
use serde::Serialize;

use crate::utils::path::log_dir;

/// 日志目录占用统计（返回给设置页展示）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogStats {
    /// 日志目录绝对路径
    pub dir: String,
    /// 目录内所有文件的总字节数
    pub size_bytes: u64,
    /// 日志文件个数
    pub file_count: usize,
}

/// 清理历史日志的结果。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearLogsResult {
    /// 删除的文件数
    pub removed: usize,
    /// 释放的字节数
    pub freed_bytes: u64,
}

/// 统计日志目录占用：路径 + 总字节数 + 文件数。
///
/// 目录不存在（从未写过日志）时返回大小 0、文件数 0，不报错——
/// 故返回值直接是 `LogStats` 而非 `Result`（本函数无失败路径）。
#[must_use]
pub fn log_stats() -> LogStats {
    let dir = log_dir();
    let mut size_bytes = 0u64;
    let mut file_count = 0usize;
    // read_dir 失败（目录不存在）时跳过，视为空目录
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    size_bytes += meta.len();
                    file_count += 1;
                }
            }
        }
    }
    LogStats {
        dir: dir.to_string_lossy().into_owned(),
        size_bytes,
        file_count,
    }
}

/// 清理历史日志：保留当天 `gitview.log.<today>`，删除更早的滚动文件。
///
/// 只处理以 `gitview.log` 开头的文件，避免误删目录下的其它文件。
/// 删除失败的单个文件被静默跳过，整体不报错，故直接返回 `ClearLogsResult`。
#[must_use]
pub fn clear_old_logs() -> ClearLogsResult {
    let dir = log_dir();
    // 当天文件名（与 tracing-appender daily 滚动命名一致），予以保留
    let keep = format!("gitview.log.{}", Local::now().format("%Y-%m-%d"));
    let mut removed = 0usize;
    let mut freed_bytes = 0u64;
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            // 仅删本应用的日志文件，且跳过当天那一份
            if name.starts_with("gitview.log") && name != keep {
                let size = entry.metadata().map_or(0, |m| m.len());
                if std::fs::remove_file(entry.path()).is_ok() {
                    removed += 1;
                    freed_bytes += size;
                }
            }
        }
    }
    ClearLogsResult {
        removed,
        freed_bytes,
    }
}
