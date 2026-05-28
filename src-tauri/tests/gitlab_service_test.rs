//! GitLab Provider 集成测试。
//!
//! 覆盖：
//!   - 200 成功（PRIVATE-TOKEN 头）
//!   - 401 → TokenInvalid
//!   - 403 → Forbidden
//!   - Token 不泄漏
//!   - derive_gitlab_api_url 表驱动用例（虽然单元测试已覆盖，这里再次确认契约）

#![allow(clippy::unwrap_used, clippy::expect_used)]

use gitview_lib::errors::GitViewError;
use gitview_lib::services::gitlab_service::{
    derive_gitlab_api_url, GitLabClientConfig, GitLabProvider,
};
use gitview_lib::services::provider::GitHostingProvider;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TEST_TOKEN: &str = "glpat-abcd1234efgh5678ijkl"; // allow-token-pattern: 测试样本

fn client_config(api_base: &str) -> GitLabClientConfig {
    GitLabClientConfig {
        api_base_url: api_base.to_string(),
        allow_invalid_certs: false,
        proxy_url: None,
        request_timeout_seconds: Some(5),
    }
}

#[tokio::test]
async fn get_current_user_success() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/user"))
        .and(header("private-token", TEST_TOKEN))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": 42,
            "username": "alice",
            "name": "Alice Liddell",
            "avatar_url": "https://avatars.example.com/alice.png",
            "web_url": "https://gitlab.example.com/alice",
        })))
        .mount(&server)
        .await;

    let provider = GitLabProvider::new(client_config(&server.uri()), TEST_TOKEN.to_string())
        .expect("应能构造 GitLabProvider");
    let profile = provider.get_current_user().await.expect("应成功");
    assert_eq!(profile.username, "alice");
    assert_eq!(profile.display_name.as_deref(), Some("Alice Liddell"));
}

#[tokio::test]
async fn unauthorized_maps_to_token_invalid() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/user"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;
    let provider = GitLabProvider::new(client_config(&server.uri()), TEST_TOKEN.to_string())
        .expect("应能构造");
    let err = provider.get_current_user().await.unwrap_err();
    assert!(matches!(err, GitViewError::TokenInvalid));
}

#[tokio::test]
async fn forbidden_maps_to_forbidden() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/user"))
        .respond_with(ResponseTemplate::new(403))
        .mount(&server)
        .await;
    let provider = GitLabProvider::new(client_config(&server.uri()), TEST_TOKEN.to_string())
        .expect("应能构造");
    let err = provider.get_current_user().await.unwrap_err();
    assert!(matches!(err, GitViewError::Forbidden));
}

#[tokio::test]
async fn error_message_does_not_leak_token() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/user"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    let provider = GitLabProvider::new(client_config(&server.uri()), TEST_TOKEN.to_string())
        .expect("应能构造");
    let err = provider.get_current_user().await.unwrap_err();
    let display = err.to_string();
    assert!(
        !display.contains(TEST_TOKEN),
        "错误消息不应包含 token 明文：{display}"
    );
}

/// derive_gitlab_api_url 表驱动测试（与单元测试互补，强调集成层契约稳定）
#[test]
fn derive_api_url_table_driven() {
    let cases: &[(&str, &str)] = &[
        ("https://gitlab.com", "https://gitlab.com/api/v4"),
        ("https://gitlab.com/", "https://gitlab.com/api/v4"),
        (
            "https://gitlab.company.com:8443",
            "https://gitlab.company.com:8443/api/v4",
        ),
        ("http://10.0.0.5", "http://10.0.0.5/api/v4"),
        (
            "https://code.company.com/gitlab",
            "https://code.company.com/gitlab/api/v4",
        ),
        (
            "https://code.company.com/gitlab/",
            "https://code.company.com/gitlab/api/v4",
        ),
    ];
    for (input, expected) in cases {
        let got = derive_gitlab_api_url(input).expect("应成功");
        assert_eq!(got, *expected, "input={input}");
    }
}
