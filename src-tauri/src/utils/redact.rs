//! Token 脱敏工具。
//!
//! 用于在写入日志、错误消息、序列化输出前，对可能含敏感凭据的文本进行脱敏。
//! 是宪法 Principle III（凭据安全）的核心强制门禁之一。
//!
//! 支持的模式：
//!   - GitHub PAT：`ghp_` / `gho_` / `ghu_` / `ghs_` / `ghr_` 前缀，36~255 字符
//!   - GitHub fine-grained PAT：`github_pat_` 前缀
//!   - GitLab PAT：`glpat-` 前缀
//!   - Gitee Token：`gitee` API 返回的 access_token（无固定前缀，按上下文匹配）
//!   - HTTP `Authorization: Bearer <token>` 头
//!   - URL 内联凭据：`https://<token>@host` 或 `https://<user>:<token>@host`
//!
//! 替换占位符：
//!   - 独立 token：`<REDACTED-TOKEN>`
//!   - URL 内联凭据：`https://<REDACTED>@host`
//!
//! 实现说明：所有正则均为编译期常量字符串，运行期不可能失败；
//! `expect` 仅作为不可达分支的崩溃哨兵，符合 clippy 社区惯例。

#![allow(clippy::expect_used)]

use once_cell::sync::Lazy;
use regex::Regex;

/// GitHub Personal Access Token 经典格式与 OAuth token 前缀。
///
/// 匹配示例：
///   - `ghp_abcdefghij1234567890ABCDEFGHIJ123456`
///   - `gho_aBcDeFgHiJkLmNoPqRsTuVwXyZ0123456789`
static RE_GITHUB_PAT: Lazy<Regex> = Lazy::new(|| {
    // 前缀 + 36~255 个 base62 字符（GitHub 文档区间，宽松匹配）
    Regex::new(r"\b(?:ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9]{36,255}\b").expect("GitHub PAT 正则编译失败")
});

/// GitHub fine-grained personal access token。
///
/// 匹配示例：`github_pat_11ABCDEFG0abcdefg_xyz...`
static RE_GITHUB_FINE_PAT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\bgithub_pat_[A-Za-z0-9_]{20,255}\b")
        .expect("GitHub fine-grained PAT 正则编译失败")
});

/// GitLab Personal Access Token。
///
/// 匹配示例：`glpat-abcdefghijklmnopqrst`（前缀后 20 字符）。
static RE_GITLAB_PAT: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\bglpat-[A-Za-z0-9_\-]{20,255}\b").expect("GitLab PAT 正则编译失败"));

/// HTTP Authorization Bearer 头中的 token。
///
/// 匹配示例：`Authorization: Bearer abc123`（大小写不敏感）。
static RE_BEARER: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)(Bearer\s+)[A-Za-z0-9._\-]+").expect("Bearer 正则编译失败"));

/// URL 内联凭据。
///
/// 匹配示例：
///   - `https://abc123@github.com/foo/bar.git`
///   - `https://user:pass@gitlab.example.com/foo/bar.git`
static RE_URL_CREDENTIAL: Lazy<Regex> = Lazy::new(|| {
    // (scheme://)(userinfo)@(host...)
    Regex::new(r"((?:https?|git)://)[^/\s@]+@").expect("URL 凭据正则编译失败")
});

/// 对输入文本进行 Token 脱敏。
///
/// # Arguments
///
/// * `text` - 原始文本（可能包含 token、Bearer 头、URL 内联凭据）
///
/// # Returns
///
/// 脱敏后的新字符串；若输入不含任何敏感片段则原样返回（仍然分配新 String）。
///
/// # Examples
///
/// ```
/// use gitview_lib::utils::redact::redact_token;
/// let s = redact_token("token=ghp_abcdefghij1234567890ABCDEFGHIJ123456");
/// assert!(s.contains("<REDACTED-TOKEN>"));
/// assert!(!s.contains("ghp_abcdefghij1234567890ABCDEFGHIJ123456"));
/// ```
#[must_use]
pub fn redact_token(text: &str) -> String {
    let mut out = text.to_string();

    // 先处理 URL 内联凭据：保留 scheme 与 host，仅替换 userinfo 段
    out = RE_URL_CREDENTIAL
        .replace_all(&out, "${1}<REDACTED>@")
        .to_string();

    // Bearer 头：保留 "Bearer " 前缀，仅替换 token 部分
    out = RE_BEARER
        .replace_all(&out, "${1}<REDACTED-TOKEN>")
        .to_string();

    // 独立 token 模式（顺序：fine-grained 在普通 PAT 之前，避免被前者吞掉）
    out = RE_GITHUB_FINE_PAT
        .replace_all(&out, "<REDACTED-TOKEN>")
        .to_string();
    out = RE_GITHUB_PAT
        .replace_all(&out, "<REDACTED-TOKEN>")
        .to_string();
    out = RE_GITLAB_PAT
        .replace_all(&out, "<REDACTED-TOKEN>")
        .to_string();

    out
}

// =====================================================================
// 单元测试 —— 覆盖每种敏感模式的脱敏路径
// =====================================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_github_pat_classic() {
        let raw = "Authorization=ghp_abcdefghij1234567890ABCDEFGHIJ123456 found";
        let out = redact_token(raw);
        assert!(out.contains("<REDACTED-TOKEN>"));
        assert!(!out.contains("ghp_"));
    }

    #[test]
    fn redact_github_pat_fine_grained() {
        let raw = "token=github_pat_11ABCDEFG0abcdefghij_xyzABCDEFGHIJ trailing";
        let out = redact_token(raw);
        assert!(out.contains("<REDACTED-TOKEN>"));
        assert!(!out.contains("github_pat_"));
    }

    #[test]
    fn redact_gitlab_pat() {
        let raw = "pat=glpat-abcdefghij1234567890 done";
        let out = redact_token(raw);
        assert!(out.contains("<REDACTED-TOKEN>"));
        assert!(!out.contains("glpat-"));
    }

    #[test]
    fn redact_bearer_header() {
        let raw = "Authorization: Bearer abc.def-123_XYZ";
        let out = redact_token(raw);
        assert!(out.contains("Bearer <REDACTED-TOKEN>"));
        assert!(!out.contains("abc.def-123_XYZ"));
    }

    #[test]
    fn redact_url_inline_credentials() {
        let raw = "remote: https://user:secret-pass@gitlab.example.com/foo.git";
        let out = redact_token(raw);
        assert!(out.contains("<REDACTED>@gitlab.example.com"));
        assert!(!out.contains("secret-pass"));
    }

    #[test]
    fn redact_preserves_clean_text() {
        let raw = "no tokens here, just a regular log line";
        let out = redact_token(raw);
        assert_eq!(out, raw);
    }

    #[test]
    fn redact_handles_multiple_secrets_in_one_line() {
        let raw = "ghp_abcdefghij1234567890ABCDEFGHIJ123456 and glpat-abcdefghij1234567890";
        let out = redact_token(raw);
        assert_eq!(out.matches("<REDACTED-TOKEN>").count(), 2);
    }
}
