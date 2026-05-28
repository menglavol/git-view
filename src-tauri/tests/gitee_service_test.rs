//! Gitee Provider 集成测试。
//!
//! 覆盖两种认证模式：
//!   - Header `Authorization: token <token>`（默认）
//!   - Query `?access_token=<token>`
//!
//! 以及统一的错误码映射与 token 不泄漏断言。

#![allow(clippy::unwrap_used, clippy::expect_used)]

use gitview_lib::errors::GitViewError;
use gitview_lib::services::gitee_service::{GiteeAuthMode, GiteeProvider};
use gitview_lib::services::provider::GitHostingProvider;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TEST_TOKEN: &str = "gitee_test_token_for_integration_testing";

#[tokio::test]
async fn get_current_user_with_header_auth() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/user"))
        .and(header("authorization", format!("token {TEST_TOKEN}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "login": "alice",
            "name": "Alice",
            "avatar_url": "https://gitee.com/avatars/alice.png",
        })))
        .mount(&server)
        .await;

    let provider = GiteeProvider::new(
        Some(server.uri()),
        TEST_TOKEN.to_string(),
        None,
        GiteeAuthMode::Header,
    )
    .expect("应能构造");
    let profile = provider.get_current_user().await.expect("应成功");
    assert_eq!(profile.username, "alice");
    assert_eq!(profile.display_name.as_deref(), Some("Alice"));
}

#[tokio::test]
async fn get_current_user_with_query_auth() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/user"))
        .and(query_param("access_token", TEST_TOKEN))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "login": "bob",
            "name": null,
            "avatar_url": null,
        })))
        .mount(&server)
        .await;

    let provider = GiteeProvider::new(
        Some(server.uri()),
        TEST_TOKEN.to_string(),
        None,
        GiteeAuthMode::Query,
    )
    .expect("应能构造");
    let profile = provider.get_current_user().await.expect("应成功");
    assert_eq!(profile.username, "bob");
    assert!(profile.display_name.is_none());
    assert!(profile.avatar_url.is_none());
}

#[tokio::test]
async fn unauthorized_maps_to_token_invalid() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/user"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;
    let provider = GiteeProvider::new(
        Some(server.uri()),
        TEST_TOKEN.to_string(),
        None,
        GiteeAuthMode::Header,
    )
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
    let provider = GiteeProvider::new(
        Some(server.uri()),
        TEST_TOKEN.to_string(),
        None,
        GiteeAuthMode::Header,
    )
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
    let provider = GiteeProvider::new(
        Some(server.uri()),
        TEST_TOKEN.to_string(),
        None,
        GiteeAuthMode::Header,
    )
    .expect("应能构造");
    let err = provider.get_current_user().await.unwrap_err();
    let display = err.to_string();
    assert!(
        !display.contains(TEST_TOKEN),
        "错误消息不应包含 token 明文：{display}"
    );
}
