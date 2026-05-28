//! Clone token 安全端到端测试。
//!
//! 验证宪法 Principle III 与 spec SC-009 / SC-010：
//!   - HTTPS Clone 时 token 通过 GIT_ASKPASS 临时脚本注入，不在命令行参数上
//!   - 临时脚本在任务结束后被立即删除（drop guard）
//!   - 错误消息经 redact_token 脱敏后不残留 token 字节
//!   - 任何输出（含 URL 内联凭据 / Bearer 头 / PAT 前缀）都被脱敏
//!
//! AskpassGuard 内部字段为私有以防误用；针对 guard 路径与生命周期的细粒度
//! 断言已经在 `services::git_cli_service` 内部 #[cfg(test)] 模块覆盖。
//! 本文件聚焦端到端可观测的脱敏契约。

#![allow(clippy::unwrap_used, clippy::expect_used)]

use gitview_lib::services::git_cli_service::{AskpassGuard, CredentialInjection};
use gitview_lib::utils::redact::redact_token;

const FAKE_TOKEN: &str = "ghp_AbCdEfGhIjKlMnOpQrStUvWxYz0123456789ABCD"; // allow-token-pattern: 测试样本

#[test]
fn askpass_guard_can_be_constructed_and_dropped() {
    let cred = CredentialInjection {
        username: "alice".to_string(),
        token: FAKE_TOKEN.to_string(),
    };
    let guard = AskpassGuard::create(&cred).unwrap();
    drop(guard);
}

#[test]
fn redact_strips_token_from_url_credentials() {
    let raw =
        format!("error sending request to https://oauth2:{FAKE_TOKEN}@github.com/foo/bar.git");
    let safe = redact_token(&raw);
    assert!(!safe.contains(FAKE_TOKEN));
    assert!(!safe.contains("ghp_"));
    assert!(safe.contains("<REDACTED>@github.com"));
}

#[test]
fn redact_strips_token_from_bearer_header() {
    let raw = format!("Authorization: Bearer {FAKE_TOKEN}");
    let safe = redact_token(&raw);
    assert!(!safe.contains(FAKE_TOKEN));
    assert!(safe.contains("Bearer <REDACTED-TOKEN>"));
}

#[test]
fn redact_strips_standalone_pat_in_log_line() {
    let raw = format!("clone failed: token={FAKE_TOKEN}, retrying with cached credentials");
    let safe = redact_token(&raw);
    assert!(!safe.contains(FAKE_TOKEN));
    assert!(!safe.contains("ghp_"));
    assert!(safe.contains("<REDACTED-TOKEN>"));
}

#[test]
fn redact_handles_mixed_secrets_in_one_message() {
    let raw = format!(
        "GitHub token={FAKE_TOKEN} and remote=https://user:secret-pw@gitlab.com/x.git", // allow-token-pattern: 测试样本
    );
    let safe = redact_token(&raw);
    assert!(!safe.contains(FAKE_TOKEN));
    assert!(!safe.contains("secret-pw"));
    assert!(safe.contains("<REDACTED-TOKEN>"));
    assert!(safe.contains("<REDACTED>@gitlab.com"));
}
