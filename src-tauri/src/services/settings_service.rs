//! 应用设置服务（US7 / T099）。
//!
//! 负责 `settings` 表的读写。设计要点:
//!   - 沿用项目「无状态自由函数 + 显式传 `&DbPool`」模式（对齐 log_service）,不引入单例。
//!   - 设置按四组（general / git / network / external_tools）各存一行,
//!     value 为该组结构的 JSON 字符串;新增字段无需迁移。
//!   - **宽容读取**:某组缺行或 JSON 反序列化失败时回退 `Default`,
//!     保证旧库、手工改库或新增字段场景下设置加载不会整体失败。
//!
//! 安全:设置不含 token（凭据走 keyring,见 credential_service）,故无需脱敏。

use rusqlite::OptionalExtension;
use serde::{de::DeserializeOwned, Serialize};

use crate::db::pool::DbPool;
use crate::errors::Result;
use crate::models::settings::{
    ExternalToolsSettings, GeneralSettings, GitSettings, NetworkSettings, Settings,
};
use crate::utils::time::now_iso8601;

// 各分组在 settings 表中的 key。集中定义避免散落的魔法字符串拼写漂移。
const KEY_GENERAL: &str = "general";
const KEY_GIT: &str = "git";
const KEY_NETWORK: &str = "network";
const KEY_EXTERNAL_TOOLS: &str = "external_tools";

/// 读取某一组设置;缺行或反序列化失败时回退 `T::default()`。
///
/// 宽容回退是有意为之:设置是「尽力提供合理默认」而非强一致数据,
/// 任何读取异常都不应让用户连设置页都打不开。新增字段时旧 JSON 缺该字段,
/// serde 默认会报错,这里统一兜底为默认值实现向前兼容。
fn read_group<T: DeserializeOwned + Default>(pool: &DbPool, key: &str) -> Result<T> {
    // 闭包内只做 SQL（rusqlite 错误经 ? 自动转 GitViewError）;
    // optional() 把「无此行」转成 Ok(None) 而非 NotFound 错误。
    let raw: Option<String> = pool.with_conn(|conn| {
        let value = conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                rusqlite::params![key],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        Ok(value)
    })?;

    // 反序列化在闭包外:失败时不传播,直接回退默认（见上方 WHY）。
    Ok(raw
        .and_then(|json| serde_json::from_str::<T>(&json).ok())
        .unwrap_or_default())
}

/// 写入某一组设置（upsert:存在则更新 value 与 updated_at）。
fn write_group<T: Serialize>(pool: &DbPool, key: &str, value: &T) -> Result<()> {
    // 序列化在闭包外:serde_json::Error 经 ? 转 GitViewError::Internal。
    let json = serde_json::to_string(value)?;
    let now = now_iso8601();
    pool.with_conn(move |conn| {
        conn.execute(
            "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at = ?3",
            rusqlite::params![key, json, now],
        )?;
        Ok(())
    })
}

/// 读取通用设置组。
pub fn get_general(pool: &DbPool) -> Result<GeneralSettings> {
    read_group(pool, KEY_GENERAL)
}

/// 写入通用设置组。
pub fn set_general(pool: &DbPool, value: &GeneralSettings) -> Result<()> {
    write_group(pool, KEY_GENERAL, value)
}

/// 读取 Git 设置组。
pub fn get_git(pool: &DbPool) -> Result<GitSettings> {
    read_group(pool, KEY_GIT)
}

/// 写入 Git 设置组。
pub fn set_git(pool: &DbPool, value: &GitSettings) -> Result<()> {
    write_group(pool, KEY_GIT, value)
}

/// 读取网络设置组。
pub fn get_network(pool: &DbPool) -> Result<NetworkSettings> {
    read_group(pool, KEY_NETWORK)
}

/// 写入网络设置组。
pub fn set_network(pool: &DbPool, value: &NetworkSettings) -> Result<()> {
    write_group(pool, KEY_NETWORK, value)
}

/// 读取外部工具设置组。
pub fn get_external_tools(pool: &DbPool) -> Result<ExternalToolsSettings> {
    read_group(pool, KEY_EXTERNAL_TOOLS)
}

/// 写入外部工具设置组。
pub fn set_external_tools(pool: &DbPool, value: &ExternalToolsSettings) -> Result<()> {
    write_group(pool, KEY_EXTERNAL_TOOLS, value)
}

/// 读取完整设置快照（聚合四组）。
pub fn get_settings(pool: &DbPool) -> Result<Settings> {
    Ok(Settings {
        general: get_general(pool)?, // 通用组：目录/协议/并发/主题/语言
        git: get_git(pool)?,         // Git 组：路径/身份/pull-push 策略
        network: get_network(pool)?, // 网络组：代理/超时
        external_tools: get_external_tools(pool)?, // 外部工具组：编辑器/终端/文件管理器
    })
}

/// 原子写入完整设置快照。
///
/// 四组在**单个事务**内写入:避免「保存一半」——比如写完 general 但 network 失败,
/// 会让用户看到半套生效的诡异状态。用 `unchecked_transaction` 在共享 `&Connection`
/// 上开事务（with_conn 只给不可变引用,无法用需要 &mut 的 `transaction()`）。
pub fn update_settings(pool: &DbPool, settings: &Settings) -> Result<()> {
    // 序列化放闭包外,任一组失败立即返回,不进入事务。
    let general = serde_json::to_string(&settings.general)?;
    let git = serde_json::to_string(&settings.git)?;
    let network = serde_json::to_string(&settings.network)?;
    let external = serde_json::to_string(&settings.external_tools)?;
    let now = now_iso8601();

    pool.with_conn(move |conn| {
        // 事务未显式 commit 时,Transaction 析构会自动回滚,保证原子性。
        let tx = conn.unchecked_transaction()?;
        for (key, value) in [
            (KEY_GENERAL, &general),
            (KEY_GIT, &git),
            (KEY_NETWORK, &network),
            (KEY_EXTERNAL_TOOLS, &external),
        ] {
            tx.execute(
                "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at = ?3",
                rusqlite::params![key, value, now],
            )?;
        }
        tx.commit()?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::db::migrations::run_pending_migrations;
    use crate::models::settings::{CloneProtocol, Theme};

    /// 建一个已迁移的临时库（settings 表由 001_init 建出）。
    fn make_db() -> (DbPool, tempfile::NamedTempFile) {
        let tmp = tempfile::NamedTempFile::new().unwrap(); // 临时库文件，测试结束自动清理
        let pool = DbPool::new(tmp.path()).unwrap(); // 基于临时文件建连接池
        run_pending_migrations(&pool).unwrap(); // 跑迁移，建出 settings 表
        (pool, tmp) // 返回 tmp 句柄让调用方持有，否则文件提前删除
    }

    /// 验收:从未写入时,读取应返回默认值（宽容回退,不报错）。
    #[test]
    fn get_returns_default_when_absent() {
        let (pool, _t) = make_db();
        let g = get_general(&pool).unwrap();
        // 默认并发 3、主题 Auto —— 证明回退到 GeneralSettings::default()
        assert_eq!(g.default_concurrency, 3);
        assert!(matches!(g.theme, Theme::Auto));
    }

    /// 验收:set 后 get 能读回（往返一致）。
    #[test]
    fn set_then_get_roundtrip() {
        let (pool, _t) = make_db();
        let g = GeneralSettings {
            default_concurrency: 8,                     // 改并发以验证写回
            default_clone_protocol: CloneProtocol::Ssh, // 改协议以验证写回
            ..GeneralSettings::default()
        };
        set_general(&pool, &g).unwrap(); // 写入
        let back = get_general(&pool).unwrap(); // 读回
        assert_eq!(back.default_concurrency, 8);
        assert!(matches!(back.default_clone_protocol, CloneProtocol::Ssh));
    }

    /// 验收:update_settings 一次写四组,get_settings 读回全部一致。
    #[test]
    fn update_settings_writes_all_groups() {
        let (pool, _t) = make_db();
        let settings = Settings {
            network: NetworkSettings {
                api_timeout_secs: 99,
                ..NetworkSettings::default()
            },
            git: GitSettings {
                user_name: Some("alice".to_string()),
                ..GitSettings::default()
            },
            ..Settings::default()
        };
        update_settings(&pool, &settings).unwrap();

        let back = get_settings(&pool).unwrap();
        assert_eq!(back.network.api_timeout_secs, 99);
        assert_eq!(back.git.user_name.as_deref(), Some("alice"));
        // 未改动的组仍是默认值
        assert_eq!(back.general.default_concurrency, 3);
    }

    /// 验收:同一 key 重复 set 走 upsert,不会插入重复行。
    #[test]
    fn set_twice_upserts_single_row() {
        let (pool, _t) = make_db();
        set_network(&pool, &NetworkSettings::default()).unwrap(); // 第一次写入默认网络组
                                                                  // 用结构体更新语法构造改了 clone 超时的网络组（避免 field_reassign_with_default）
        let n = NetworkSettings {
            clone_timeout_secs: 600,
            ..NetworkSettings::default()
        };
        set_network(&pool, &n).unwrap(); // 第二次写同一 key，应走 upsert 而非新插入

        let count: i64 = pool
            .with_conn(|conn| {
                let c = conn
                    .query_row(
                        "SELECT COUNT(*) FROM settings WHERE key = ?1",
                        rusqlite::params![KEY_NETWORK],
                        |row| row.get(0),
                    )
                    .optional()?
                    .unwrap_or(0);
                Ok(c)
            })
            .unwrap();
        assert_eq!(count, 1, "upsert 不应产生重复行");
        assert_eq!(get_network(&pool).unwrap().clone_timeout_secs, 600);
    }
}
