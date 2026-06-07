//! GitHub Provider 集成测试。
//!
//! 使用 wiremock 启动本地 mock HTTP 服务器，覆盖核心路径：
//!   - 200 成功路径返回正确 UserProfile
//!   - 401 → TokenInvalid
//!   - 403 → Forbidden
//!   - 404 → NotFound
//!   - Token 不出现在错误消息中（脱敏门禁）

#![allow(clippy::unwrap_used, clippy::expect_used)]

use gitview_lib::errors::GitViewError;
use gitview_lib::models::repository::{CreateRepoRequest, Visibility};
use gitview_lib::services::github_service::GitHubProvider;
use gitview_lib::services::provider::GitHostingProvider;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TEST_TOKEN: &str = "ghp_aaaabbbbccccddddeeeeffffgggghhhhiiii"; // allow-token-pattern: 测试样本

fn make_provider(base: &str) -> GitHubProvider {
    GitHubProvider::new(Some(base.to_string()), TEST_TOKEN.to_string(), None)
        .expect("应能构造 GitHubProvider")
}

/// create_repository：201 成功，响应正确映射为 RemoteRepository
#[tokio::test]
async fn create_repository_success() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/user/repos"))
        .and(header("authorization", format!("Bearer {TEST_TOKEN}")))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": 555,
            "full_name": "octocat/new-repo",
            "name": "new-repo",
            "owner": { "login": "octocat" },
            "description": "demo",
            "default_branch": "main",
            "private": true,
            "html_url": "https://github.com/octocat/new-repo",
            "clone_url": "https://github.com/octocat/new-repo.git",
            "ssh_url": "git@github.com:octocat/new-repo.git",
        })))
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let req = CreateRepoRequest {
        name: "new-repo".to_string(),
        description: Some("demo".to_string()),
        visibility: Visibility::Private,
    };
    let repo = provider
        .create_repository(&req, "acc-1")
        .await
        .expect("应成功");
    assert_eq!(repo.account_id, "acc-1");
    assert_eq!(repo.remote_id, "555");
    assert_eq!(repo.full_name, "octocat/new-repo");
    assert!(matches!(repo.visibility, Visibility::Private));
    assert_eq!(repo.clone_url, "https://github.com/octocat/new-repo.git");
}

/// create_repository：422 且响应含 "already exists" → RepoNameTaken（前端提示改名）
#[tokio::test]
async fn create_repository_conflict_maps_to_name_taken() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/user/repos"))
        .respond_with(ResponseTemplate::new(422).set_body_json(serde_json::json!({
            "message": "Repository creation failed.",
            "errors": [{ "message": "name already exists on this account" }],
        })))
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let req = CreateRepoRequest {
        name: "dup".to_string(),
        description: None,
        visibility: Visibility::Private,
    };
    let err = provider.create_repository(&req, "acc-1").await.unwrap_err();
    assert!(matches!(err, GitViewError::RepoNameTaken));
}

#[tokio::test]
async fn get_current_user_success() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/user"))
        .and(header("authorization", format!("Bearer {TEST_TOKEN}")))
        .and(header("x-github-api-version", "2022-11-28"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "login": "octocat",
            "name": "The Octocat",
            "avatar_url": "https://avatars.example.com/octocat.png",
        })))
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let profile = provider.get_current_user().await.expect("应成功");
    assert_eq!(profile.username, "octocat");
    assert_eq!(profile.display_name.as_deref(), Some("The Octocat"));
}

#[tokio::test]
async fn unauthorized_maps_to_token_invalid() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/user"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;
    let provider = make_provider(&server.uri());
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
    let provider = make_provider(&server.uri());
    let err = provider.get_current_user().await.unwrap_err();
    assert!(matches!(err, GitViewError::Forbidden));
}

#[tokio::test]
async fn not_found_maps_to_not_found() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/user"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    let provider = make_provider(&server.uri());
    let err = provider.get_current_user().await.unwrap_err();
    assert!(matches!(err, GitViewError::NotFound(_)));
}

#[tokio::test]
async fn error_message_does_not_leak_token() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/user"))
        .respond_with(ResponseTemplate::new(500).set_body_string("oops"))
        .mount(&server)
        .await;
    let provider = make_provider(&server.uri());
    let err = provider.get_current_user().await.unwrap_err();
    let display = err.to_string();
    assert!(
        !display.contains(TEST_TOKEN),
        "错误消息不应包含 token 明文：{display}"
    );
}
