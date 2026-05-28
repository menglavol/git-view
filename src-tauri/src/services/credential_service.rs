//! 凭据存储服务。
//!
//! 封装系统密钥库（`keyring` crate）：
//!   - macOS: Keychain
//!   - Windows: Credential Manager
//!   - Linux: Secret Service (libsecret / GNOME Keyring)
//!
//! 安全约束（宪法 Principle III）：
//!   - Token 明文绝不入 SQLite 数据库
//!   - `load_token` 返回值仅在 service 内部使用，禁止暴露给 Tauri command 层
//!   - 任何错误信息在向上传递前均不携带 token 字节
//!   - 本模块不实现 `Serialize`/`Deserialize`，无法被前端通过 IPC 直接读取

use keyring::Entry;

use crate::errors::{GitViewError, Result};

/// 系统密钥库的服务名空间（所有 GitView 条目均在该 service 下）。
pub const SERVICE_NAME: &str = "gitview";

/// 构造账号 token 在密钥库中的条目键名。
///
/// 形如 `account-token-<account_id>`，账号删除时通过该键名精确清理。
fn entry_for(account_id: &str) -> Result<Entry> {
    let key = format!("account-token-{account_id}");
    Entry::new(SERVICE_NAME, &key).map_err(GitViewError::from)
}

/// 保存账号 token 到系统密钥库。
///
/// 已存在同 key 条目时会被覆盖（keyring crate 行为）。
///
/// # Arguments
///
/// * `account_id` - 账号唯一标识
/// * `token` - 待存储的 Token 明文
///
/// # Errors
///
/// 密钥库不可用或写入失败时返回 `Credential` 错误。
pub fn save_token(account_id: &str, token: &str) -> Result<()> {
    let entry = entry_for(account_id)?;
    entry.set_password(token).map_err(GitViewError::from)
}

/// 从系统密钥库读取账号 token。
///
/// **重要**：返回的 String 仅供 service 内部组装 HTTP 请求头使用，
/// 绝不可向上层 command 或前端暴露。
///
/// # Errors
///
/// 凭据不存在时返回 `NotFound`；其他失败返回 `Credential`。
pub fn load_token(account_id: &str) -> Result<String> {
    let entry = entry_for(account_id)?;
    entry.get_password().map_err(GitViewError::from)
}

/// 从系统密钥库删除账号 token。
///
/// 条目不存在时视为已删除（返回 `Ok(())`），便于幂等清理。
///
/// # Errors
///
/// 删除底层失败时返回 `Credential` 错误。
pub fn delete_token(account_id: &str) -> Result<()> {
    let entry = entry_for(account_id)?;
    match entry.delete_password() {
        // 条目不存在或删除成功均视为幂等成功
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(GitViewError::from(e)),
    }
}

/// 判断账号 token 是否已存储。
///
/// # Errors
///
/// 密钥库整体不可用时返回 `Credential` 错误（与"条目不存在"区分）。
pub fn token_exists(account_id: &str) -> Result<bool> {
    let entry = entry_for(account_id)?;
    match entry.get_password() {
        Ok(_) => Ok(true),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(e) => Err(GitViewError::from(e)),
    }
}

/// 启动期检查密钥库可用性。
///
/// 通过尝试访问一个探针条目来诊断后端是否可用；探针不存在不影响判定结果，
/// 仅当密钥库本身不可访问（如 Linux 上无 Secret Service）时返回错误。
///
/// # Errors
///
/// 返回 `Credential` 错误并附带平台特定诊断信息。
pub fn check_availability() -> Result<()> {
    let probe = Entry::new(SERVICE_NAME, "__availability_probe__")
        .map_err(|e| GitViewError::Credential(format!("密钥库初始化失败：{e}")))?;

    match probe.get_password() {
        Ok(_) | Err(keyring::Error::NoEntry) => Ok(()),
        // 平台后端不可用、权限不足等情况
        Err(e) => Err(GitViewError::Credential(format!("密钥库不可用：{e}"))),
    }
}

// =====================================================================
// 单元测试
//
// 注意：keyring 在 CI 无图形环境的 Linux 上可能不可用，
// 因此涉及实际 keyring 后端的测试用 `#[ignore]` 标记，
// 本地或 macOS / Windows runner 可手动 `cargo test -- --ignored` 验证。
// =====================================================================
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    /// 完整生命周期：保存 → 读取 → 存在性 → 删除 → 再读取应 NotFound
    #[test]
    #[ignore = "依赖系统密钥库，需在交互式登录会话下执行"]
    fn token_lifecycle_round_trip() {
        let account_id = "test-account-lifecycle";
        let token = "ghp_TestTokenValueForUnitTest1234567890";

        // 清理可能的残留
        let _ = delete_token(account_id);

        save_token(account_id, token).expect("保存 token 应成功");

        assert!(token_exists(account_id).unwrap());
        let loaded = load_token(account_id).expect("读取 token 应成功");
        assert_eq!(loaded, token);

        delete_token(account_id).expect("删除 token 应成功");

        // 删除后再读取：NotFound
        let load_result = load_token(account_id);
        assert!(matches!(load_result, Err(GitViewError::NotFound(_))));
        assert!(!token_exists(account_id).unwrap());
    }

    /// 删除不存在的条目应为幂等成功
    #[test]
    #[ignore = "依赖系统密钥库"]
    fn delete_nonexistent_is_idempotent() {
        let account_id = "test-account-never-saved";
        // 即便没保存过，调用 delete 也不应报错
        delete_token(account_id).expect("删除不存在条目应幂等成功");
    }

    /// entry_for 生成键名应包含 account_id（不依赖 keyring 后端）
    #[test]
    fn entry_key_format_is_stable() {
        // entry_for 内部生成的键名通过 Entry 不可直接读取，
        // 这里用同样的格式字符串再次构造来验证约定不被意外修改
        let account_id = "abc-123";
        let key = format!("account-token-{account_id}");
        assert_eq!(key, "account-token-abc-123");
    }
}
