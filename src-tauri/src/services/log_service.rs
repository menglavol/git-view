//! 日志服务占位实现。
//!
//! 本模块在 US1 阶段提供一个最小可用的 `record_error` 接口，
//! 其内部强制对消息执行 `redact_token` 脱敏，作为宪法 Principle III
//! 的全局防御层（即便上游忘记脱敏，到达本函数后也会被截获）。
//!
//! US6（Phase 8）将扩展为完整的操作日志写入数据库 + tracing 集成，
//! 届时本函数将转换为 `account_service` / `git_service` 等的标准日志出口。

use crate::utils::redact::redact_token;

/// 记录一次操作错误。
///
/// 当前实现：以 WARN 级别写入 tracing。消息会先经过 `redact_token` 脱敏。
///
/// # Arguments
///
/// * `operation` - 操作名（如 `add_account` / `test_connection` / `clone`）
/// * `message` - 原始错误消息（可能含 token，会被脱敏后再记录）
pub fn record_error(operation: &str, message: &str) {
    let safe = redact_token(message);
    tracing::warn!(operation = operation, "{}", safe);
}

/// 对错误消息执行一次脱敏并返回。
///
/// 供 service 层在向上传递错误前显式调用：`return Err(GitViewError::Network(scrub(&raw)))`。
#[must_use]
pub fn scrub(message: &str) -> String {
    redact_token(message)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    /// 注入伪造 PAT，scrub 后不应保留原 token 字节
    #[test]
    fn scrub_strips_github_pat() {
        let raw = "fetch failed for ghp_abcdefghij1234567890ABCDEFGHIJ123456"; // allow-token-pattern: 测试样本
        let cleaned = scrub(raw);
        assert!(cleaned.contains("<REDACTED-TOKEN>"));
        assert!(!cleaned.contains("ghp_abcdefghij1234567890ABCDEFGHIJ123456")); // allow-token-pattern: 测试样本
    }

    /// 注入 GitLab PAT，scrub 后不应保留原 token 字节
    #[test]
    fn scrub_strips_gitlab_pat() {
        let raw = "401 with token glpat-abcdefghij1234567890"; // allow-token-pattern: 测试样本
        let cleaned = scrub(raw);
        assert!(cleaned.contains("<REDACTED-TOKEN>"));
        assert!(!cleaned.contains("glpat-abcdefghij1234567890")); // allow-token-pattern: 测试样本
    }

    /// 注入 URL 内联凭据，scrub 后不应保留密码部分
    #[test]
    fn scrub_strips_url_credentials() {
        let raw = "clone error: https://user:secret123@gitlab.example.com/x.git"; // allow-token-pattern: 测试样本
        let cleaned = scrub(raw);
        assert!(cleaned.contains("<REDACTED>@gitlab.example.com"));
        assert!(!cleaned.contains("secret123"));
    }

    /// 干净文本应原样返回
    #[test]
    fn scrub_preserves_clean_message() {
        let raw = "operation completed successfully";
        assert_eq!(scrub(raw), raw);
    }
}
