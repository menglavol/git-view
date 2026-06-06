//! 账号业务服务。
//!
//! 提供账号 CRUD、测试连接、默认账号管理、启用/禁用切换、同步互斥控制。
//!
//! 关键设计：
//!   - **事务 + keyring 协同**：`add_account` 先调用 provider 测试连接成功后，
//!     在一个 DB 事务内插入 accounts/gitlab_instance_configs 行，并保存 token
//!     到 keyring；任一步骤失败时统一回滚（含 keyring 条目清理）
//!   - **账号粒度互斥锁**：`AccountServiceState::syncing` 记录正在同步的账号集合；
//!     同账号重复触发返回 `BusyAccount`，跨账号并行
//!   - **默认账号唯一性**：set_default 时使用 `UPDATE accounts SET is_default = 0`
//!     + `UPDATE accounts SET is_default = 1 WHERE id = ?` 两步原子操作
//!   - **安全约束**：token 明文仅在 service 内部短暂持有，构造完成后立即
//!     `keyring::save_token` 移交至系统密钥库

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use uuid::Uuid;

use crate::db::pool::DbPool;
use crate::errors::{GitViewError, Result};
use crate::models::account::{
    Account, AccountUpdate, AddAccountPayload, AddGitLabInstanceConfigPayload, CloneProtocolPref,
    ConnectionStatus, GitLabInstanceConfig, GitPlatform, TestConnectionPayload,
};
use crate::models::repository::Visibility;
use crate::models::settings::NetworkSettings;
use crate::services::credential_service;
use crate::services::gitee_service::{GiteeAuthMode, GiteeProvider};
use crate::services::github_service::GitHubProvider;
use crate::services::gitlab_service::{derive_gitlab_api_url, GitLabClientConfig, GitLabProvider};
use crate::services::provider::{GitHostingProvider, UserProfile};
use crate::services::proxy::{resolve_proxy, ProxyDecision};
use crate::services::settings_service;

/// 账号服务的可克隆运行时状态。
///
/// 互斥锁集合记录"正在同步中的 account_id"；通过 `Arc` 在多个 Tauri command
/// 调用间共享。
#[derive(Clone, Default)]
pub struct AccountServiceState {
    syncing: Arc<Mutex<HashSet<String>>>,
}

impl AccountServiceState {
    /// 创建空状态实例。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 尝试为指定账号占用同步槽位。
    ///
    /// 占用成功返回 `SyncGuard`（drop 时自动释放）；
    /// 同账号已在同步中返回 `BusyAccount`。
    pub fn try_acquire_sync(&self, account_id: &str) -> Result<SyncGuard<'_>> {
        {
            let mut set = self
                .syncing
                .lock()
                .map_err(|e| GitViewError::Internal(format!("同步集合锁损坏：{e}")))?;
            if set.contains(account_id) {
                return Err(GitViewError::BusyAccount(account_id.to_string()));
            }
            set.insert(account_id.to_string());
        } // 显式提前释放锁，避免持有跨返回值的 MutexGuard
        Ok(SyncGuard {
            state: self,
            account_id: account_id.to_string(),
        })
    }
}

/// 同步槽位 RAII 守护。
///
/// drop 时自动从 `syncing` 集合移除对应 account_id，确保异常路径也能释放。
pub struct SyncGuard<'a> {
    state: &'a AccountServiceState,
    account_id: String,
}

impl Drop for SyncGuard<'_> {
    fn drop(&mut self) {
        // 锁损坏时也强制清理状态：忽略错误，避免 panic in drop
        if let Ok(mut set) = self.state.syncing.lock() {
            set.remove(&self.account_id);
        }
    }
}

// =====================================================================
// 添加账号
// =====================================================================

/// 添加账号。
///
/// 流程：
///   1. 推导/校验 API 地址
///   2. 实例化 Provider，调用 `get_current_user` 测试连接（失败即返回）
///   3. 开启 DB 事务：插入 accounts（含 enabled = true、is_default 视情况）
///   4. 若为自建 GitLab，插入 gitlab_instance_configs
///   5. 保存 token 到 keyring；失败时回滚事务
///   6. 返回完整 Account
///
/// **安全约束**：token 仅在本函数内的 `payload` 与 provider 构造时持有，
/// 保存到 keyring 后调用方可释放（payload 由 Tauri command 反序列化后由
/// drop 销毁）。
#[allow(clippy::too_many_lines)]
pub async fn add_account(pool: &DbPool, payload: AddAccountPayload) -> Result<Account> {
    let api_base_url = resolve_api_base_url(
        payload.platform,
        &payload.web_base_url,
        payload.api_base_url.as_deref(),
    )?;

    // 1) 测试连接（读全局网络设置注入代理,与后续同步行为一致）
    let net = settings_service::get_network(pool)?;
    let profile = test_connection_inner(
        payload.platform,
        &payload.web_base_url,
        &api_base_url,
        &payload.token,
        payload.instance_config.as_ref(),
        &net,
    )
    .await?;

    // 2) 写入数据库（事务）
    let account_id = Uuid::new_v4().to_string();
    let token_key = format!("account-token-{account_id}");
    let now_iso = Utc::now().to_rfc3339();
    let platform_str = serialize_platform(payload.platform);

    // 检查是否需要将本账号设为默认（当前无任何账号时）
    let should_be_default = pool.with_conn(|conn| {
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM accounts", [], |row| row.get(0))
            .map_err(GitViewError::from)?;
        Ok(count == 0)
    })?;

    // 把闭包外仍需用到的字段先单独取出（如 token），其余 payload 字段 move 进闭包
    let token_for_keyring = payload.token;
    let instance_config_to_insert = payload.instance_config;
    let web_base_url = payload.web_base_url;
    let api_base_url_for_db = api_base_url;
    let remark = payload.remark;
    let display_name = profile.display_name;
    let avatar_url = profile.avatar_url;
    let username = profile.username;
    let acc_id = account_id.clone();
    let tk = token_key;
    let now_iso_for_db = now_iso;

    pool.with_conn(move |conn| {
        conn.execute_batch("BEGIN;")?;

        let insert_account_result = conn.execute(
            "INSERT INTO accounts (
                id, platform, web_base_url, api_base_url, username,
                display_name, avatar_url, token_key,
                is_default, enabled, remark,
                created_at, updated_at, last_sync_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, NULL)",
            params![
                acc_id,
                platform_str,
                web_base_url,
                api_base_url_for_db,
                username,
                display_name,
                avatar_url,
                tk,
                i64::from(should_be_default),
                1_i64, // enabled = true
                remark,
                now_iso_for_db,
                now_iso_for_db,
            ],
        );

        if let Err(e) = insert_account_result {
            let _ = conn.execute_batch("ROLLBACK;");
            return Err(GitViewError::from(e));
        }

        // GitLab 实例配置（如有）
        if let Some(cfg) = instance_config_to_insert {
            let cfg_id = Uuid::new_v4().to_string();
            let proto_str = serialize_clone_protocol(cfg.default_clone_protocol);
            let timeout_db = cfg
                .request_timeout_seconds
                .and_then(|v| i64::try_from(v).ok());
            let insert_cfg = conn.execute(
                "INSERT INTO gitlab_instance_configs (
                    id, account_id, web_base_url, api_base_url,
                    allow_insecure_http, allow_invalid_certs, use_system_proxy,
                    proxy_url, request_timeout_seconds, default_clone_protocol,
                    ssh_host_alias, api_path_prefix,
                    last_connection_status, last_connection_error,
                    created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                params![
                    cfg_id,
                    acc_id,
                    web_base_url,
                    api_base_url_for_db,
                    i64::from(cfg.allow_insecure_http),
                    i64::from(cfg.allow_invalid_certs),
                    i64::from(cfg.use_system_proxy),
                    cfg.proxy_url,
                    timeout_db,
                    proto_str,
                    cfg.ssh_host_alias,
                    cfg.api_path_prefix,
                    "success", // 测试连接已通过
                    Option::<String>::None,
                    now_iso_for_db,
                    now_iso_for_db,
                ],
            );
            if let Err(e) = insert_cfg {
                let _ = conn.execute_batch("ROLLBACK;");
                return Err(GitViewError::from(e));
            }
        }

        conn.execute_batch("COMMIT;")?;
        Ok(())
    })?;

    // 3) 保存 token 到 keyring；失败时回滚数据库
    if let Err(e) = credential_service::save_token(&account_id, &token_for_keyring) {
        // 数据库已经 commit，需要补偿删除
        let _ = pool.with_conn(|conn| {
            conn.execute("DELETE FROM accounts WHERE id = ?1", params![account_id])
                .map_err(GitViewError::from)
        });
        return Err(e);
    }

    // 4) 返回完整 Account
    load_account_by_id(pool, &account_id)
}

// =====================================================================
// 测试连接
// =====================================================================

/// 测试账号连接（不写入数据库）。
///
/// `pool` 仅用于读取全局网络设置以注入代理,不产生写副作用。
pub async fn test_account_connection(
    pool: &DbPool,
    payload: TestConnectionPayload,
) -> Result<UserProfile> {
    let api_base_url = resolve_api_base_url(
        payload.platform,
        &payload.web_base_url,
        payload.api_base_url.as_deref(),
    )?;
    let net = settings_service::get_network(pool)?;
    test_connection_inner(
        payload.platform,
        &payload.web_base_url,
        &api_base_url,
        &payload.token,
        payload.instance_config.as_ref(),
        &net,
    )
    .await
}

/// 内部：根据平台构造 Provider 并调用 get_current_user。
///
/// `net` 为全局网络设置:GitHub/Gitee 无账号级代理,直接用全局兜底;
/// GitLab 自建实例的账号级代理优先,未配时回退全局（见 `provider_proxy`）。
async fn test_connection_inner(
    platform: GitPlatform,
    _web_base_url: &str,
    api_base_url: &str,
    token: &str,
    instance_config: Option<&AddGitLabInstanceConfigPayload>,
    net: &NetworkSettings,
) -> Result<UserProfile> {
    let provider: Box<dyn GitHostingProvider> = match platform {
        GitPlatform::Github => Box::new(GitHubProvider::new(
            Some(api_base_url.to_string()),
            token.to_string(),
            provider_proxy(net, None, false),
        )?),
        GitPlatform::Gitlab => {
            let cfg = instance_config.map_or_else(
                || GitLabClientConfig {
                    api_base_url: api_base_url.to_string(),
                    allow_invalid_certs: false,
                    proxy_url: provider_proxy(net, None, false),
                    request_timeout_seconds: None,
                },
                |c| GitLabClientConfig {
                    api_base_url: api_base_url.to_string(),
                    allow_invalid_certs: c.allow_invalid_certs,
                    proxy_url: provider_proxy(net, c.proxy_url.as_deref(), c.use_system_proxy),
                    request_timeout_seconds: c.request_timeout_seconds,
                },
            );
            Box::new(GitLabProvider::new(cfg, token.to_string())?)
        }
        GitPlatform::Gitee => Box::new(GiteeProvider::new(
            Some(api_base_url.to_string()),
            token.to_string(),
            provider_proxy(net, None, false),
            GiteeAuthMode::Header,
        )?),
    };
    provider.get_current_user().await
}

// =====================================================================
// 查询
// =====================================================================

/// 列出所有账号（默认账号优先，其次按创建时间升序）。
pub fn list_accounts(pool: &DbPool) -> Result<Vec<Account>> {
    pool.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, platform, web_base_url, api_base_url, username,
                    display_name, avatar_url, token_key,
                    is_default, enabled, remark,
                    created_at, updated_at, last_sync_at
             FROM accounts
             ORDER BY is_default DESC, created_at ASC",
        )?;
        let rows = stmt.query_map([], row_to_account)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    })
}

/// 通过 ID 加载单个账号（找不到时返回 `NotFound`）。
pub fn load_account_by_id(pool: &DbPool, id: &str) -> Result<Account> {
    pool.with_conn(|conn| {
        conn.query_row(
            "SELECT id, platform, web_base_url, api_base_url, username,
                    display_name, avatar_url, token_key,
                    is_default, enabled, remark,
                    created_at, updated_at, last_sync_at
             FROM accounts WHERE id = ?1",
            params![id],
            row_to_account,
        )
        .map_err(GitViewError::from)
    })
}

/// 读取账号关联的 GitLab 实例配置（无配置时返回 `None`）。
pub fn get_gitlab_instance_config(
    pool: &DbPool,
    account_id: &str,
) -> Result<Option<GitLabInstanceConfig>> {
    pool.with_conn(|conn| {
        let row = conn
            .query_row(
                "SELECT id, account_id, web_base_url, api_base_url,
                        allow_insecure_http, allow_invalid_certs, use_system_proxy,
                        proxy_url, request_timeout_seconds, default_clone_protocol,
                        ssh_host_alias, api_path_prefix,
                        last_connection_status, last_connection_error,
                        created_at, updated_at
                 FROM gitlab_instance_configs WHERE account_id = ?1",
                params![account_id],
                row_to_instance_config,
            )
            .optional()
            .map_err(GitViewError::from)?;
        Ok(row)
    })
}

// =====================================================================
// 更新 / 启用切换
// =====================================================================

/// 更新账号的可选字段。
///
/// `fields` 中为 `Some(...)` 的字段会被更新；`None` 字段保持不变。
/// 当 `enabled` 切换为 `false` 且该账号为默认账号时，自动将默认转移到
/// 剩余 enabled 账号中创建最早的一个。
pub fn update_account(pool: &DbPool, id: &str, fields: AccountUpdate) -> Result<Account> {
    let now_iso = Utc::now().to_rfc3339();
    let id_owned = id.to_string();

    pool.with_conn(move |conn| {
        // 加载当前账号
        let current: Account = conn
            .query_row(
                "SELECT id, platform, web_base_url, api_base_url, username,
                        display_name, avatar_url, token_key,
                        is_default, enabled, remark,
                        created_at, updated_at, last_sync_at
                 FROM accounts WHERE id = ?1",
                params![id_owned],
                row_to_account,
            )
            .map_err(GitViewError::from)?;

        let new_display = fields.display_name.or(current.display_name);
        let new_avatar = fields.avatar_url.or(current.avatar_url);
        let new_remark = fields.remark.or(current.remark);
        let new_enabled = fields.enabled.unwrap_or(current.enabled);

        conn.execute_batch("BEGIN;")?;
        let upd_result = conn.execute(
            "UPDATE accounts
             SET display_name = ?1, avatar_url = ?2, remark = ?3, enabled = ?4,
                 updated_at = ?5
             WHERE id = ?6",
            params![
                new_display,
                new_avatar,
                new_remark,
                i64::from(new_enabled),
                now_iso,
                id_owned,
            ],
        );
        if let Err(e) = upd_result {
            let _ = conn.execute_batch("ROLLBACK;");
            return Err(GitViewError::from(e));
        }

        // 若禁用且当前为默认，转移默认到剩余 enabled 最早一个
        if !new_enabled && current.is_default {
            let next_default: Option<String> = conn
                .query_row(
                    "SELECT id FROM accounts WHERE enabled = 1 AND id <> ?1
                     ORDER BY created_at ASC LIMIT 1",
                    params![id_owned],
                    |row| row.get(0),
                )
                .optional()
                .map_err(GitViewError::from)?;

            let clear = conn.execute(
                "UPDATE accounts SET is_default = 0 WHERE is_default = 1",
                [],
            );
            if let Err(e) = clear {
                let _ = conn.execute_batch("ROLLBACK;");
                return Err(GitViewError::from(e));
            }
            if let Some(target_id) = next_default {
                let set_new = conn.execute(
                    "UPDATE accounts SET is_default = 1, updated_at = ?1 WHERE id = ?2",
                    params![now_iso, target_id],
                );
                if let Err(e) = set_new {
                    let _ = conn.execute_batch("ROLLBACK;");
                    return Err(GitViewError::from(e));
                }
            }
        }

        conn.execute_batch("COMMIT;")?;
        Ok(())
    })?;

    load_account_by_id(pool, id)
}

/// 启用/禁用切换的语法糖（等价于 `update_account` 设置 `enabled` 字段）。
pub fn set_account_enabled(pool: &DbPool, id: &str, enabled: bool) -> Result<Account> {
    update_account(
        pool,
        id,
        AccountUpdate {
            enabled: Some(enabled),
            ..AccountUpdate::default()
        },
    )
}

// =====================================================================
// 默认账号设置
// =====================================================================

/// 把指定账号设为默认。
///
/// 目标账号必须 `enabled = true`，否则返回 `Forbidden` 错误。
pub fn set_default_account(pool: &DbPool, id: &str) -> Result<()> {
    let now_iso = Utc::now().to_rfc3339();
    let id_owned = id.to_string();
    pool.with_conn(move |conn| {
        // 目标账号必须存在且启用
        let enabled: i64 = conn
            .query_row(
                "SELECT enabled FROM accounts WHERE id = ?1",
                params![id_owned],
                |row| row.get(0),
            )
            .map_err(GitViewError::from)?;
        if enabled == 0 {
            return Err(GitViewError::Forbidden);
        }

        conn.execute_batch("BEGIN;")?;
        let clear = conn.execute(
            "UPDATE accounts SET is_default = 0 WHERE is_default = 1",
            [],
        );
        if let Err(e) = clear {
            let _ = conn.execute_batch("ROLLBACK;");
            return Err(GitViewError::from(e));
        }
        let set = conn.execute(
            "UPDATE accounts SET is_default = 1, updated_at = ?1 WHERE id = ?2",
            params![now_iso, id_owned],
        );
        if let Err(e) = set {
            let _ = conn.execute_batch("ROLLBACK;");
            return Err(GitViewError::from(e));
        }
        conn.execute_batch("COMMIT;")?;
        Ok(())
    })
}

// =====================================================================
// 删除账号
// =====================================================================

/// 删除账号。
///
/// 流程：
///   1. keyring 删除 token（幂等）
///   2. 数据库删除 accounts 行（外键级联清理 gitlab_instance_configs / 远程仓库）
///   3. 若该账号为默认账号，将默认转移到剩余 enabled 最早一个
pub fn delete_account(pool: &DbPool, id: &str) -> Result<()> {
    // 1) 先删 keyring（即便后续 DB 删除失败，凭据残留也不暴露）
    credential_service::delete_token(id)?;

    let id_owned = id.to_string();
    pool.with_conn(move |conn| {
        let was_default: i64 = conn
            .query_row(
                "SELECT is_default FROM accounts WHERE id = ?1",
                params![id_owned],
                |row| row.get(0),
            )
            .optional()
            .map_err(GitViewError::from)?
            .unwrap_or(0);

        conn.execute_batch("BEGIN;")?;
        let del = conn.execute("DELETE FROM accounts WHERE id = ?1", params![id_owned]);
        if let Err(e) = del {
            let _ = conn.execute_batch("ROLLBACK;");
            return Err(GitViewError::from(e));
        }

        if was_default == 1 {
            let next_default: Option<String> = conn
                .query_row(
                    "SELECT id FROM accounts WHERE enabled = 1
                     ORDER BY created_at ASC LIMIT 1",
                    [],
                    |row| row.get(0),
                )
                .optional()
                .map_err(GitViewError::from)?;
            if let Some(target_id) = next_default {
                let now_iso = Utc::now().to_rfc3339();
                let set_new = conn.execute(
                    "UPDATE accounts SET is_default = 1, updated_at = ?1 WHERE id = ?2",
                    params![now_iso, target_id],
                );
                if let Err(e) = set_new {
                    let _ = conn.execute_batch("ROLLBACK;");
                    return Err(GitViewError::from(e));
                }
            }
        }
        conn.execute_batch("COMMIT;")?;
        Ok(())
    })
}

// =====================================================================
// 同步入口（具体实现在 US2 完善，本任务仅提供 stub）
// =====================================================================

/// 同步账号下的远程仓库。
///
/// 流程：
///   1. 占用同步互斥锁
///   2. 从 keyring 加载 token，构造 Provider
///   3. 循环分页拉取 list_repositories 直到 has_next = false
///   4. 标记该账号下现有缓存为 stale
///   5. Upsert 拉取到的仓库（按 account_id + remote_id 唯一键）
///   6. 清理仍为 stale 的旧记录（保留 is_favorite 的条目）
///   7. 更新 accounts.last_sync_at
///
/// 返回同步到的仓库总数。
pub async fn sync_account_repositories(
    state: &AccountServiceState,
    pool: &DbPool,
    account_id: &str,
) -> Result<usize> {
    let _guard = state.try_acquire_sync(account_id)?;

    let account = load_account_by_id(pool, account_id)?;
    let token = credential_service::load_token(account_id)?;

    let provider: Box<dyn GitHostingProvider> = build_provider(pool, &account, &token)?;

    // 同步起始时刻：后续拉取动作均发生在此之后，故本批仓库的 synced_at 必定 >= 该值。
    // 用它（而非拉取完成后的时刻）作为清理旧记录的基准，避免误删刚 upsert 的新记录。
    let sync_started_at = Utc::now().to_rfc3339();

    let mut all_repos = Vec::new();
    let mut page = 1_u32;
    let per_page = 100_u32;

    loop {
        let result = provider
            .list_repositories(page, per_page, account_id)
            .await?;
        all_repos.extend(result.items);
        if !result.has_next {
            break;
        }
        page += 1;
        if page > 200 {
            break;
        }
    }

    let total = all_repos.len();
    let now_iso = Utc::now().to_rfc3339();
    let acc_id = account_id.to_string();

    pool.with_conn(move |conn| {
        conn.execute_batch("BEGIN;")?;

        for repo in &all_repos {
            let vis_str = serialize_visibility(repo.visibility);
            let pushed_str = repo.last_pushed_at.map(|dt| dt.to_rfc3339());
            let synced_str = repo.synced_at.to_rfc3339();

            conn.execute(
                "INSERT INTO remote_repositories (
                    id, account_id, platform, remote_id, full_name, name, owner,
                    description, visibility, default_branch, html_url, ssh_url,
                    clone_url, is_favorite, last_pushed_at, synced_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
                ON CONFLICT(account_id, remote_id) DO UPDATE SET
                    full_name = excluded.full_name,
                    name = excluded.name,
                    owner = excluded.owner,
                    description = excluded.description,
                    visibility = excluded.visibility,
                    default_branch = excluded.default_branch,
                    html_url = excluded.html_url,
                    ssh_url = excluded.ssh_url,
                    clone_url = excluded.clone_url,
                    last_pushed_at = excluded.last_pushed_at,
                    synced_at = excluded.synced_at",
                params![
                    repo.id,
                    repo.account_id,
                    serialize_platform(repo.platform),
                    repo.remote_id,
                    repo.full_name,
                    repo.name,
                    repo.owner,
                    repo.description,
                    vis_str,
                    repo.default_branch,
                    repo.html_url,
                    repo.ssh_url,
                    repo.clone_url,
                    i64::from(repo.is_favorite),
                    pushed_str,
                    synced_str,
                ],
            )
            .map_err(GitViewError::from)?;
        }

        // 清理本次同步未覆盖到的旧记录（保留收藏的）。
        // 基准用 sync_started_at（拉取前时刻），而非拉取完成后的时刻：
        // 新记录 synced_at 来自 provider 拉取时刻，必 >= sync_started_at，不会被误删。
        conn.execute(
            "DELETE FROM remote_repositories
             WHERE account_id = ?1 AND synced_at < ?2 AND is_favorite = 0",
            params![acc_id, sync_started_at],
        )
        .map_err(GitViewError::from)?;

        // 更新 last_sync_at
        conn.execute(
            "UPDATE accounts SET last_sync_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![now_iso, acc_id],
        )
        .map_err(GitViewError::from)?;

        conn.execute_batch("COMMIT;")?;
        Ok(())
    })?;

    Ok(total)
}

// =====================================================================
// 辅助函数
// =====================================================================

/// 根据平台与 Web 地址推导/校验 API 地址。
fn resolve_api_base_url(
    platform: GitPlatform,
    web_base_url: &str,
    explicit_api: Option<&str>,
) -> Result<String> {
    if let Some(explicit) = explicit_api {
        if !explicit.trim().is_empty() {
            return Ok(explicit.trim_end_matches('/').to_string());
        }
    }
    match platform {
        GitPlatform::Github => Ok("https://api.github.com".to_string()),
        GitPlatform::Gitlab => derive_gitlab_api_url(web_base_url),
        GitPlatform::Gitee => Ok("https://gitee.com/api/v5".to_string()),
    }
}

/// 把账号级与全局网络设置合并成传给 Provider 的代理 URL。
///
/// Provider 的 `new` 只收 `Option<String>`,故把三态 `ProxyDecision` 压成两态:
///   - `Explicit(url)` → `Some(url)`,强制走该代理
///   - `System` / `None` → `None`,交由 reqwest 默认（读环境变量,即跟随系统;
///     直连场景无环境代理变量时即直连）
///
/// V1 直连与跟随系统在「无环境代理变量」时行为一致,差异可忽略,故不为
/// `None` 单独表达「强制直连」。
fn provider_proxy(
    net: &NetworkSettings,
    account_proxy: Option<&str>,
    account_use_system: bool,
) -> Option<String> {
    match resolve_proxy(net, account_proxy, account_use_system) {
        ProxyDecision::Explicit(url) => Some(url),
        ProxyDecision::System | ProxyDecision::None => None,
    }
}

/// 根据账号信息构造对应平台的 Provider 实例。
///
/// 代理:读全局网络设置并与账号级代理（仅自建 GitLab 有）合并后传给 provider,
/// 让设置里的全局代理对所有平台 API 调用生效——GitHub/Gitee 无账号级代理,
/// 之前恒直连,这里补上全局兜底。
fn build_provider(
    pool: &DbPool,
    account: &Account,
    token: &str,
) -> Result<Box<dyn GitHostingProvider>> {
    let net = settings_service::get_network(pool)?;
    match account.platform {
        GitPlatform::Github => Ok(Box::new(GitHubProvider::new(
            Some(account.api_base_url.clone()),
            token.to_string(),
            provider_proxy(&net, None, false),
        )?)),
        GitPlatform::Gitlab => {
            let instance_cfg = get_gitlab_instance_config(pool, &account.id)?;
            let cfg = instance_cfg.map_or_else(
                || GitLabClientConfig {
                    api_base_url: account.api_base_url.clone(),
                    allow_invalid_certs: false,
                    proxy_url: provider_proxy(&net, None, false),
                    request_timeout_seconds: None,
                },
                |c| GitLabClientConfig {
                    api_base_url: account.api_base_url.clone(),
                    allow_invalid_certs: c.allow_invalid_certs,
                    proxy_url: provider_proxy(&net, c.proxy_url.as_deref(), c.use_system_proxy),
                    request_timeout_seconds: c.request_timeout_seconds,
                },
            );
            Ok(Box::new(GitLabProvider::new(cfg, token.to_string())?))
        }
        GitPlatform::Gitee => Ok(Box::new(GiteeProvider::new(
            Some(account.api_base_url.clone()),
            token.to_string(),
            provider_proxy(&net, None, false),
            GiteeAuthMode::Header,
        )?)),
    }
}

const fn serialize_visibility(v: Visibility) -> &'static str {
    match v {
        Visibility::Public => "public",
        Visibility::Private => "private",
        Visibility::Internal => "internal",
    }
}

/// 把 `GitPlatform` 序列化为数据库存储的 snake_case 字符串。
const fn serialize_platform(p: GitPlatform) -> &'static str {
    match p {
        GitPlatform::Github => "github",
        GitPlatform::Gitlab => "gitlab",
        GitPlatform::Gitee => "gitee",
    }
}

/// 反序列化数据库 platform 字段为 `GitPlatform`。
fn deserialize_platform(s: &str) -> Result<GitPlatform> {
    match s {
        "github" => Ok(GitPlatform::Github),
        "gitlab" => Ok(GitPlatform::Gitlab),
        "gitee" => Ok(GitPlatform::Gitee),
        other => Err(GitViewError::Internal(format!("未知平台标识：{other}"))),
    }
}

const fn serialize_clone_protocol(p: CloneProtocolPref) -> &'static str {
    match p {
        CloneProtocolPref::Https => "https",
        CloneProtocolPref::Ssh => "ssh",
    }
}

fn deserialize_clone_protocol(s: &str) -> CloneProtocolPref {
    match s {
        "ssh" => CloneProtocolPref::Ssh,
        _ => CloneProtocolPref::Https,
    }
}

fn deserialize_connection_status(s: &str) -> ConnectionStatus {
    match s {
        "success" => ConnectionStatus::Success,
        "failed" => ConnectionStatus::Failed,
        _ => ConnectionStatus::Unknown,
    }
}

/// 从 SQLite 行映射到 `Account`。
fn row_to_account(row: &rusqlite::Row<'_>) -> rusqlite::Result<Account> {
    let platform_str: String = row.get("platform")?;
    let platform = deserialize_platform(&platform_str).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            )),
        )
    })?;
    let created_at_str: String = row.get("created_at")?;
    let updated_at_str: String = row.get("updated_at")?;
    let last_sync_str: Option<String> = row.get("last_sync_at")?;

    let parse_dt = |s: &str| -> rusqlite::Result<chrono::DateTime<Utc>> {
        chrono::DateTime::parse_from_rfc3339(s)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        e.to_string(),
                    )),
                )
            })
    };

    let last_sync = match last_sync_str {
        Some(s) => Some(parse_dt(&s)?),
        None => None,
    };

    Ok(Account {
        id: row.get("id")?,
        platform,
        web_base_url: row.get("web_base_url")?,
        api_base_url: row.get("api_base_url")?,
        username: row.get("username")?,
        display_name: row.get("display_name")?,
        avatar_url: row.get("avatar_url")?,
        token_key: row.get("token_key")?,
        is_default: row.get::<_, i64>("is_default")? != 0,
        enabled: row.get::<_, i64>("enabled")? != 0,
        remark: row.get("remark")?,
        created_at: parse_dt(&created_at_str)?,
        updated_at: parse_dt(&updated_at_str)?,
        last_sync_at: last_sync,
    })
}

/// 从 SQLite 行映射到 `GitLabInstanceConfig`。
fn row_to_instance_config(row: &rusqlite::Row<'_>) -> rusqlite::Result<GitLabInstanceConfig> {
    let created_at_str: String = row.get("created_at")?;
    let updated_at_str: String = row.get("updated_at")?;
    let parse_dt = |s: &str| -> rusqlite::Result<chrono::DateTime<Utc>> {
        chrono::DateTime::parse_from_rfc3339(s)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        e.to_string(),
                    )),
                )
            })
    };
    let proto_str: String = row.get("default_clone_protocol")?;
    let status_str: String = row.get("last_connection_status")?;

    Ok(GitLabInstanceConfig {
        id: row.get("id")?,
        account_id: row.get("account_id")?,
        web_base_url: row.get("web_base_url")?,
        api_base_url: row.get("api_base_url")?,
        allow_insecure_http: row.get::<_, i64>("allow_insecure_http")? != 0,
        allow_invalid_certs: row.get::<_, i64>("allow_invalid_certs")? != 0,
        use_system_proxy: row.get::<_, i64>("use_system_proxy")? != 0,
        proxy_url: row.get("proxy_url")?,
        request_timeout_seconds: row
            .get::<_, Option<i64>>("request_timeout_seconds")?
            .and_then(|v| u64::try_from(v).ok()),
        default_clone_protocol: deserialize_clone_protocol(&proto_str),
        ssh_host_alias: row.get("ssh_host_alias")?,
        api_path_prefix: row.get("api_path_prefix")?,
        last_connection_status: deserialize_connection_status(&status_str),
        last_connection_error: row.get("last_connection_error")?,
        created_at: parse_dt(&created_at_str)?,
        updated_at: parse_dt(&updated_at_str)?,
    })
}

// =====================================================================
// 单元测试 —— 仅覆盖不依赖网络的逻辑（CRUD / 默认账号 / 互斥锁）
// =====================================================================
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    fn fresh_pool() -> DbPool {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        // 保留临时文件（避免 drop 后 SQLite 重新连接失败）
        let path = tmp.path().to_path_buf();
        let _ = tmp.keep();
        let pool = DbPool::new(&path).unwrap();
        crate::db::migrations::run_pending_migrations(&pool).unwrap();
        pool
    }

    /// 直接插入账号 row，绕过 provider 测试连接；用于 list/update/delete 测试。
    fn insert_account_direct(pool: &DbPool, id: &str, is_default: bool, enabled: bool) {
        let now = Utc::now().to_rfc3339();
        pool.with_conn(|conn| {
            conn.execute(
                "INSERT INTO accounts (id, platform, web_base_url, api_base_url, username,
                    token_key, is_default, enabled, created_at, updated_at)
                 VALUES (?1, 'github', 'https://github.com', 'https://api.github.com', ?2,
                    ?3, ?4, ?5, ?6, ?6)",
                params![
                    id,
                    format!("user-{id}"),
                    format!("account-token-{id}"),
                    i64::from(is_default),
                    i64::from(enabled),
                    now
                ],
            )
            .map_err(GitViewError::from)?;
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn list_accounts_orders_default_first() {
        let pool = fresh_pool();
        insert_account_direct(&pool, "a1", false, true);
        std::thread::sleep(std::time::Duration::from_millis(10));
        insert_account_direct(&pool, "a2", true, true);
        let list = list_accounts(&pool).unwrap();
        assert_eq!(list.len(), 2);
        // 默认账号排第一
        assert_eq!(list[0].id, "a2");
        assert!(list[0].is_default);
    }

    #[test]
    fn set_default_account_rejects_disabled() {
        let pool = fresh_pool();
        insert_account_direct(&pool, "a1", true, true);
        insert_account_direct(&pool, "a2", false, false); // 禁用
        let result = set_default_account(&pool, "a2");
        assert!(matches!(result, Err(GitViewError::Forbidden)));
    }

    #[test]
    fn disabling_default_transfers_to_next() {
        let pool = fresh_pool();
        insert_account_direct(&pool, "a1", true, true);
        std::thread::sleep(std::time::Duration::from_millis(10));
        insert_account_direct(&pool, "a2", false, true);
        let updated = set_account_enabled(&pool, "a1", false).unwrap();
        assert!(!updated.enabled);
        assert!(!updated.is_default);
        // 默认应转移到 a2
        let a2 = load_account_by_id(&pool, "a2").unwrap();
        assert!(a2.is_default);
    }

    #[test]
    fn delete_default_transfers_default() {
        let pool = fresh_pool();
        insert_account_direct(&pool, "a1", true, true);
        std::thread::sleep(std::time::Duration::from_millis(10));
        insert_account_direct(&pool, "a2", false, true);

        delete_account(&pool, "a1").unwrap();
        let list = list_accounts(&pool).unwrap();
        assert_eq!(list.len(), 1);
        assert!(list[0].is_default);
        assert_eq!(list[0].id, "a2");
    }

    #[test]
    fn busy_account_returned_when_same_id_in_use() {
        let state = AccountServiceState::new();
        let _g = state.try_acquire_sync("acc-1").unwrap();
        let again = state.try_acquire_sync("acc-1");
        assert!(matches!(again, Err(GitViewError::BusyAccount(_))));
    }

    #[test]
    fn different_accounts_can_sync_in_parallel() {
        let state = AccountServiceState::new();
        let _g1 = state.try_acquire_sync("acc-1").unwrap();
        let _g2 = state.try_acquire_sync("acc-2").unwrap();
        // 都成功获取，槽位未冲突
    }

    #[test]
    fn sync_guard_releases_on_drop() {
        let state = AccountServiceState::new();
        {
            let _g = state.try_acquire_sync("acc-1").unwrap();
        }
        // drop 后应能再次占用
        let again = state.try_acquire_sync("acc-1");
        assert!(again.is_ok());
    }

    #[test]
    fn resolve_api_base_explicit_overrides_default() {
        let s = resolve_api_base_url(
            GitPlatform::Github,
            "https://github.com",
            Some("https://x/"),
        )
        .unwrap();
        assert_eq!(s, "https://x");
    }

    #[test]
    fn resolve_api_base_github_default() {
        let s = resolve_api_base_url(GitPlatform::Github, "https://github.com", None).unwrap();
        assert_eq!(s, "https://api.github.com");
    }

    #[test]
    fn resolve_api_base_gitlab_derived() {
        let s =
            resolve_api_base_url(GitPlatform::Gitlab, "https://gitlab.example.com", None).unwrap();
        assert_eq!(s, "https://gitlab.example.com/api/v4");
    }
}
