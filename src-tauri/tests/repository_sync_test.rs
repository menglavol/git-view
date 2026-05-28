//! 远程仓库同步集成测试。
//!
//! 验证：
//!   - 多页拉取后所有仓库均落库
//!   - 二次同步收敛一致（不重复、不丢失）
//!   - 收藏标记在同步覆盖时保留
//!
//! 注意：本测试依赖系统密钥库（keyring），在无图形会话的 CI Linux 上
//! 会失败，因此统一以 `#[ignore]` 标记，本地手动运行：
//!   `cargo test --test repository_sync_test -- --ignored`

#![allow(clippy::unwrap_used, clippy::expect_used)]

use chrono::Utc;
use gitview_lib::db::migrations::run_pending_migrations;
use gitview_lib::db::pool::DbPool;
use gitview_lib::services::{
    account_service, account_service::AccountServiceState, credential_service,
};
use rusqlite::params;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TEST_TOKEN: &str = "ghp_sync_integration_token_value_xxxxx";

fn fresh_pool() -> DbPool {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    let _ = tmp.keep();
    let pool = DbPool::new(&path).unwrap();
    run_pending_migrations(&pool).unwrap();
    pool
}

fn insert_test_account(pool: &DbPool, id: &str, api_base: &str) {
    let now = Utc::now().to_rfc3339();
    pool.with_conn(|conn| {
        conn.execute(
            "INSERT INTO accounts (id, platform, web_base_url, api_base_url, username,
                token_key, is_default, enabled, created_at, updated_at)
             VALUES (?1, 'github', 'https://github.com', ?2, 'octocat',
                ?3, 1, 1, ?4, ?4)",
            params![id, api_base, format!("account-token-{id}"), now],
        )
        .map_err(gitview_lib::errors::GitViewError::from)?;
        Ok(())
    })
    .unwrap();
}

fn count_repos(pool: &DbPool, account_id: &str) -> i64 {
    pool.with_conn(|conn| {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM remote_repositories WHERE account_id = ?1",
                params![account_id],
                |row| row.get(0),
            )
            .map_err(gitview_lib::errors::GitViewError::from)?;
        Ok(count)
    })
    .unwrap()
}

fn make_repo_json(id: i64, name: &str) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "full_name": format!("octocat/{name}"),
        "name": name,
        "owner": { "login": "octocat" },
        "description": format!("Test repo {name}"),
        "default_branch": "main",
        "private": false,
        "html_url": format!("https://github.com/octocat/{name}"),
        "clone_url": format!("https://github.com/octocat/{name}.git"),
        "ssh_url": format!("git@github.com:octocat/{name}.git"),
        "language": "Rust",
        "pushed_at": "2026-01-15T10:00:00Z",
    })
}

#[tokio::test]
#[ignore = "依赖系统密钥库；本地 macOS / Windows 可手动 --ignored 运行"]
async fn sync_paginated_repositories_persists_all_pages() {
    let server = MockServer::start().await;
    let page1: Vec<_> = (1..=100)
        .map(|i| make_repo_json(i, &format!("repo-{i}")))
        .collect();
    let page2: Vec<_> = (101..=150)
        .map(|i| make_repo_json(i, &format!("repo-{i}")))
        .collect();

    Mock::given(method("GET"))
        .and(path("/user/repos"))
        .and(query_param("page", "1"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(&page1)
                .insert_header(
                    "link",
                    format!("<{}/user/repos?page=2>; rel=\"next\"", server.uri()),
                ),
        )
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/user/repos"))
        .and(query_param("page", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&page2))
        .mount(&server)
        .await;

    let pool = fresh_pool();
    let account_id = "acc-sync-paginated";
    insert_test_account(&pool, account_id, &server.uri());
    credential_service::save_token(account_id, TEST_TOKEN).unwrap();

    let state = AccountServiceState::new();
    let count = account_service::sync_account_repositories(&state, &pool, account_id)
        .await
        .expect("同步应成功");

    let _ = credential_service::delete_token(account_id);

    assert_eq!(count, 150);
    assert_eq!(count_repos(&pool, account_id), 150);
}

#[tokio::test]
#[ignore = "依赖系统密钥库；本地 macOS / Windows 可手动 --ignored 运行"]
async fn sync_twice_keeps_consistent_count_and_preserves_favorites() {
    let server = MockServer::start().await;
    let repos: Vec<_> = (1..=10)
        .map(|i| make_repo_json(i, &format!("r-{i}")))
        .collect();

    Mock::given(method("GET"))
        .and(path("/user/repos"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&repos))
        .mount(&server)
        .await;

    let pool = fresh_pool();
    let account_id = "acc-sync-favorites";
    insert_test_account(&pool, account_id, &server.uri());
    credential_service::save_token(account_id, TEST_TOKEN).unwrap();

    let state = AccountServiceState::new();

    let first = account_service::sync_account_repositories(&state, &pool, account_id)
        .await
        .unwrap();
    assert_eq!(first, 10);

    pool.with_conn(|conn| {
        conn.execute(
            "UPDATE remote_repositories SET is_favorite = 1 WHERE account_id = ?1 AND remote_id = '1'",
            params![account_id],
        )
        .map_err(gitview_lib::errors::GitViewError::from)?;
        Ok(())
    })
    .unwrap();

    let second = account_service::sync_account_repositories(&state, &pool, account_id)
        .await
        .unwrap();

    let fav: i64 = pool
        .with_conn(|conn| {
            conn.query_row(
                "SELECT is_favorite FROM remote_repositories WHERE account_id = ?1 AND remote_id = '1'",
                params![account_id],
                |row| row.get(0),
            )
            .map_err(gitview_lib::errors::GitViewError::from)
        })
        .unwrap();

    let _ = credential_service::delete_token(account_id);

    assert_eq!(second, 10);
    assert_eq!(count_repos(&pool, account_id), 10);
    assert_eq!(fav, 1, "收藏标记应在同步后保留");
}
