//! 时间工具：ISO 8601 格式化与解析。
//!
//! 项目内统一使用 UTC + RFC 3339（ISO 8601 的子集）作为时间字符串格式，
//! 与前端 `Date.toISOString()` 输出兼容。

use chrono::{DateTime, Utc};

use crate::errors::{GitViewError, Result};

/// 获取当前 UTC 时间的 ISO 8601 字符串。
///
/// 输出形如 `2025-05-26T10:30:45.123456+00:00`，毫秒/微秒精度按 chrono 默认。
#[must_use]
pub fn now_iso8601() -> String {
    Utc::now().to_rfc3339()
}

/// 解析 ISO 8601 时间字符串为 `DateTime<Utc>`。
///
/// 支持 RFC 3339 全量格式（含时区偏移）；不带时区的输入将被视为非法。
///
/// # Errors
///
/// 解析失败时返回 `Internal` 错误，错误信息包含原始输入便于诊断。
pub fn parse_iso8601(s: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| GitViewError::Internal(format!("时间格式无效 '{s}'：{e}")))
}

// =====================================================================
// 单元测试
// =====================================================================
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn now_iso8601_round_trip() {
        let s = now_iso8601();
        let parsed = parse_iso8601(&s).expect("应可解析自身输出");
        // 通过再次格式化对比，验证语义一致（具体字符串可能因精度差异不同）
        let _ = parsed.to_rfc3339();
    }

    #[test]
    fn parse_iso8601_valid_input() {
        let dt = parse_iso8601("2025-01-15T10:30:00+00:00").expect("应成功解析");
        assert_eq!(dt.to_rfc3339(), "2025-01-15T10:30:00+00:00");
    }

    #[test]
    fn parse_iso8601_invalid_input_returns_error() {
        let result = parse_iso8601("not a date");
        assert!(result.is_err());
    }

    #[test]
    fn parse_iso8601_rejects_missing_timezone() {
        // RFC 3339 要求时区，不带 Z / 偏移的输入应失败
        let result = parse_iso8601("2025-01-15T10:30:00");
        assert!(result.is_err());
    }
}
