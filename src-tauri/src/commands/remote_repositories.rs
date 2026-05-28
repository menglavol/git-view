//! 远程仓库相关 Tauri commands。

#![allow(clippy::needless_pass_by_value)]

use tauri::State;

use crate::errors::Result;
use crate::models::repository::RemoteRepository;
use crate::services::account_service;
use crate::services::repository_service::{self, RemoteRepoFilter};
use crate::AppState;

/// 查询远程仓库列表（支持多条件筛选）。
#[tauri::command]
pub fn list_remote_repositories(
    state: State<'_, AppState>,
    filter: RemoteRepoFilter,
) -> Result<Vec<RemoteRepository>> {
    repository_service::list_remote_repositories(&state.db, &filter)
}

/// 搜索远程仓库（筛选 + 关键词）。
#[tauri::command]
pub fn search_remote_repositories(
    state: State<'_, AppState>,
    keyword: String,
    filter: RemoteRepoFilter,
) -> Result<Vec<RemoteRepository>> {
    let mut f = filter;
    f.search = Some(keyword);
    repository_service::list_remote_repositories(&state.db, &f)
}

/// 刷新远程仓库（触发同步）。
#[tauri::command]
pub async fn refresh_remote_repositories(
    state: State<'_, AppState>,
    account_id: Option<String>,
) -> Result<usize> {
    if let Some(id) = account_id {
        account_service::sync_account_repositories(&state.account_service_state, &state.db, &id)
            .await
    } else {
        let accounts = account_service::list_accounts(&state.db)?;
        let mut total = 0;
        for acc in accounts {
            if acc.enabled {
                // 单账号同步失败不阻塞其他账号；累计成功的仓库数
                if let Ok(n) = account_service::sync_account_repositories(
                    &state.account_service_state,
                    &state.db,
                    &acc.id,
                )
                .await
                {
                    total += n;
                }
            }
        }
        Ok(total)
    }
}

/// 获取单个远程仓库详情。
#[tauri::command]
pub fn get_remote_repository_detail(
    state: State<'_, AppState>,
    repo_id: String,
) -> Result<RemoteRepository> {
    repository_service::get_remote_repository(&state.db, &repo_id)
}

/// 切换远程仓库收藏状态。
#[tauri::command]
pub fn toggle_favorite_remote_repository(
    state: State<'_, AppState>,
    repo_id: String,
) -> Result<bool> {
    repository_service::toggle_favorite(&state.db, &repo_id)
}
