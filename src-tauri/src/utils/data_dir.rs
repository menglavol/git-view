//! 应用数据目录的「指针文件」读写。
//!
//! 数据库与日志默认放在 `dirs::data_local_dir()/gitview`，但用户可在设置里
//! 改到自定义目录。由于所有设置都存在 DB 内、而 DB 又在数据目录里（鸡生蛋），
//! 「数据目录在哪」这个配置无法存进 DB，必须落到一个**固定不可配置位置**的
//! 指针文件：`dirs::config_dir()/gitview/datadir.json`。
//!
//! 指针文件与可迁移的 data_local_dir 解耦：即使数据搬到外置盘，指针始终留在
//! 本机用户配置区。应用启动时先读它确定真实数据目录，再据此打开 DB / 写日志。

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::errors::Result;
use crate::utils::path::ensure_dir_exists;

/// 指针文件内容：记录当前数据目录与（迁移后保留待删的）旧目录。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataDirPointer {
    /// 当前生效的数据目录绝对路径
    pub data_dir: String,
    /// 上一次迁移前的旧数据目录（保留待用户手动删除；删除后置空不再序列化）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_dir: Option<String>,
}

/// 指针文件固定路径：`<config_dir>/gitview/datadir.json`。
///
/// config_dir 缺失（极少见）时回退当前目录，保证函数总有返回值。
#[must_use]
pub fn pointer_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("gitview")
        .join("datadir.json")
}

/// 应用默认数据目录：`<data_local_dir>/gitview`（指针缺失 / 损坏时的回退）。
#[must_use]
pub fn default_data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("gitview")
}

/// 读取指针文件；缺失或解析失败一律返回 `None`（不污染启动流程）。
///
/// 首次启动从未迁移时文件不存在 → `None`；文件被外部改坏导致 JSON 解析失败
/// 也 → `None`，由 `resolve_data_dir` 回退默认目录，保证应用总能启动。
#[must_use]
pub fn read_pointer() -> Option<DataDirPointer> {
    let path = pointer_path();
    // 读不到内容（不存在 / 无权限）或内容损坏都视为「无有效指针」
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

/// 原子写入指针文件：先写同目录临时文件再 rename，避免写一半被读到。
///
/// # Errors
///
/// 创建配置目录或写 / 重命名文件失败时返回错误。
pub fn write_pointer(pointer: &DataDirPointer) -> Result<()> {
    let path = pointer_path();
    // 确保 <config_dir>/gitview 目录存在
    if let Some(parent) = path.parent() {
        ensure_dir_exists(parent)?;
    }
    let json = serde_json::to_string_pretty(pointer)?;
    // 原子写：写到同目录 .tmp 再 rename（同分区 rename 是原子操作）
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

/// 解析当前生效的数据目录。
///
/// 指针存在且其 `dataDir` 目录真实存在 → 采用指针；否则（首次启动 / 指针损坏 /
/// 指向目录已被删或拔盘）回退默认 `<data_local_dir>/gitview`。
#[must_use]
pub fn resolve_data_dir() -> PathBuf {
    if let Some(p) = read_pointer() {
        let dir = PathBuf::from(&p.data_dir);
        // 指针指向的目录必须真实存在才采用，避免误用一个已失效的路径
        if dir.is_dir() {
            return dir;
        }
    }
    default_data_dir()
}

// =====================================================================
// 单元测试
// =====================================================================

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    /// 测试：指针序列化省略 None 的 previousDir，反序列化能回读 dataDir
    #[test]
    fn test_pointer_serde_skips_none_previous() {
        let p = DataDirPointer {
            data_dir: "/tmp/data".to_string(),
            previous_dir: None,
        };
        let json = serde_json::to_string(&p).unwrap();
        // previousDir 为 None 时不应出现在 JSON 里
        assert!(!json.contains("previousDir"));
        assert!(json.contains("dataDir"));
        let back: DataDirPointer = serde_json::from_str(&json).unwrap();
        assert_eq!(back.data_dir, "/tmp/data");
        assert!(back.previous_dir.is_none());
    }

    /// 测试：含 previousDir 的指针往返序列化保持一致
    #[test]
    fn test_pointer_serde_roundtrip_with_previous() {
        let p = DataDirPointer {
            data_dir: "/new".to_string(),
            previous_dir: Some("/old".to_string()),
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: DataDirPointer = serde_json::from_str(&json).unwrap();
        assert_eq!(back.previous_dir.as_deref(), Some("/old"));
    }

    /// 测试：默认数据目录以 gitview 结尾
    #[test]
    fn test_default_data_dir_ends_with_gitview() {
        assert!(default_data_dir().ends_with("gitview"));
    }
}
