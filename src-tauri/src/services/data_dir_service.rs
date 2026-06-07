//! 数据目录迁移服务。
//!
//! 把当前数据目录（`gitview.db` + `logs/`）复制到用户指定的新目录，并更新指针
//! 文件指向新目录；旧目录**保留**待用户手动删除。运行中的 DB 连接与日志 appender
//! 启动即绑定、无法热切换，故迁移后需重启进程才生效（由前端引导）。
//!
//! 安全约定：
//!   - 复制而非移动；任一步失败都**不更新指针**，应用继续用旧目录，数据不丢。
//!   - 目标已含 `gitview.db` 直接报错拒绝，绝不覆盖既有数据。
//!   - 失败时不自动删除新目录下已复制的内容（避免破坏用户既有文件），残留副本
//!     由用户自行清理或下次迁移时被「目标已存在」校验拦截。

use std::path::{Path, PathBuf};

use serde::Serialize;
use walkdir::WalkDir;

use crate::db::pool::DbPool;
use crate::errors::{GitViewError, Result};
use crate::utils::data_dir::{read_pointer, write_pointer, DataDirPointer};
use crate::utils::path::app_data_dir;

/// 数据库主文件名（WAL/SHM 旁文件在此基础上加 `-wal`/`-shm` 后缀）。
const DB_FILE: &str = "gitview.db";

/// 迁移结果（返回前端：新目录 + 被保留的旧目录）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrateResult {
    /// 迁移后的新数据目录绝对路径
    pub new_dir: String,
    /// 被保留的旧数据目录绝对路径（待手动删除）
    pub previous_dir: String,
}

/// 旧数据目录的占用统计（供设置页「删除旧目录」展示）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OldDataDir {
    /// 旧目录绝对路径
    pub dir: String,
    /// 旧目录内所有文件的总字节数
    pub size_bytes: u64,
    /// 旧目录文件个数
    pub file_count: usize,
}

/// 把当前数据目录迁移（复制）到 `new_dir`，更新指针，旧目录保留。
///
/// 步骤：校验目标 → WAL checkpoint → 复制 DB 三件套 → 复制 logs → 写指针。
///
/// # Errors
///
/// 目标校验不通过（与当前同目录 / 位于当前目录内 / 已含 `gitview.db` / 不可写）、
/// WAL checkpoint 失败、复制失败或写指针失败时返回错误；失败时指针不动。
pub fn migrate(db: &DbPool, new_dir: &Path) -> Result<MigrateResult> {
    let current = app_data_dir();

    // ---- 1. 目标目录校验 ----
    // 不存在则先建出来，随后 canonicalize 需要真实路径
    std::fs::create_dir_all(new_dir)?;
    let new_canon = dunce::canonicalize(new_dir)
        .map_err(|e| GitViewError::PathMissing(format!("{}: {e}", new_dir.display())))?;
    // 当前目录一定存在（DB 正在使用）；canonicalize 失败兜底用原路径
    let current_canon = dunce::canonicalize(&current).unwrap_or_else(|_| current.clone());

    // 与当前目录相同 → 无意义，拒绝
    if new_canon == current_canon {
        return Err(GitViewError::PathConflict(
            "新目录与当前数据目录相同".to_string(),
        ));
    }
    // 新目录位于当前目录内部 → 复制会自包含递归，拒绝
    if new_canon.starts_with(&current_canon) {
        return Err(GitViewError::PathConflict(
            "新目录不能位于当前数据目录内部".to_string(),
        ));
    }
    // 目标已有 gitview.db → 报错拒绝（绝不覆盖既有数据）
    if new_canon.join(DB_FILE).exists() {
        return Err(GitViewError::PathConflict(format!(
            "目标目录已存在 {DB_FILE}，请选择空目录或先清理"
        )));
    }
    // 探测可写：写一个临时文件再删（捕捉权限 / 只读盘问题）
    let probe = new_canon.join(".gitview_write_probe");
    std::fs::write(&probe, b"probe")?;
    let _ = std::fs::remove_file(&probe);

    // ---- 2. WAL checkpoint：把 -wal 内容落盘并截断，保证复制到一致状态 ----
    db.with_conn(|conn| {
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .map_err(|e| GitViewError::Database(format!("WAL checkpoint 失败：{e}")))?;
        Ok(())
    })?;

    // ---- 3. 复制 DB 主文件 + WAL/SHM 旁文件 ----
    // checkpoint(TRUNCATE) 后 -wal 通常为空甚至不存在，故旁文件「存在才复制」
    for name in [DB_FILE, "gitview.db-wal", "gitview.db-shm"] {
        let src = current_canon.join(name);
        if src.exists() {
            std::fs::copy(&src, new_canon.join(name))?;
        }
    }

    // ---- 4. 递归复制 logs/ 目录（不存在则跳过）----
    let src_logs = current_canon.join("logs");
    if src_logs.is_dir() {
        copy_dir_recursive(&src_logs, &new_canon.join("logs"))?;
    }

    // ---- 5. 更新指针：dataDir=新，previousDir=旧（最后一步，前面失败则指针不动）----
    let new_str = new_canon.to_string_lossy().into_owned();
    let prev_str = current_canon.to_string_lossy().into_owned();
    write_pointer(&DataDirPointer {
        data_dir: new_str.clone(),
        previous_dir: Some(prev_str.clone()),
    })?;

    Ok(MigrateResult {
        new_dir: new_str,
        previous_dir: prev_str,
    })
}

/// 读取旧数据目录及其占用；指针无 `previousDir` 或目录已不存在时返回 `None`。
#[must_use]
pub fn old_data_dir() -> Option<OldDataDir> {
    let prev = read_pointer()?.previous_dir?;
    let dir = PathBuf::from(&prev);
    // 旧目录可能已被用户在文件系统层面删掉，此时视为无旧目录
    if !dir.is_dir() {
        return None;
    }
    let (size_bytes, file_count) = dir_usage(&dir);
    Some(OldDataDir {
        dir: prev,
        size_bytes,
        file_count,
    })
}

/// 删除旧数据目录，并清空指针的 `previousDir` 字段。
///
/// 无指针或无 `previousDir` 时幂等返回 `Ok(())`。
///
/// # Errors
///
/// 删除目录或回写指针失败时返回错误。
pub fn delete_old_data_dir() -> Result<()> {
    // 无指针 = 从未迁移，无旧目录可删
    let Some(mut pointer) = read_pointer() else {
        return Ok(());
    };
    // 取出并置空 previousDir；无值说明已删过，幂等返回
    let Some(prev) = pointer.previous_dir.take() else {
        return Ok(());
    };
    let dir = PathBuf::from(&prev);
    if dir.is_dir() {
        std::fs::remove_dir_all(&dir)?;
    }
    // 回写已清空 previousDir 的指针，之后 UI 不再显示旧目录
    write_pointer(&pointer)?;
    Ok(())
}

/// 递归统计目录占用：返回 `(总字节数, 文件数)`。
///
/// 遍历出错的条目被 `flatten` 跳过，统计尽力而为不报错。
fn dir_usage(dir: &Path) -> (u64, usize) {
    let mut size = 0u64;
    let mut count = 0usize;
    for entry in WalkDir::new(dir).into_iter().flatten() {
        if let Ok(meta) = entry.metadata() {
            if meta.is_file() {
                size += meta.len();
                count += 1;
            }
        }
    }
    (size, count)
}

/// 递归复制目录：遍历 `src` 下所有条目，按相对路径重建到 `dst`。
///
/// # Errors
///
/// 创建目录或复制文件失败时返回错误。
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    for entry in WalkDir::new(src).into_iter().flatten() {
        // entry 一定以 src 为前缀；strip 失败兜底用原路径（理论不发生）
        let rel = entry
            .path()
            .strip_prefix(src)
            .unwrap_or_else(|_| entry.path());
        let target = dst.join(rel);
        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&target)?;
        } else if entry.file_type().is_file() {
            // 兜底确保父目录存在，再复制文件内容
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}
