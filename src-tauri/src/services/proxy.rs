//! 代理决策与应用（US7 / T105）。
//!
//! 把「账号级代理设置」与「全局网络设置」合并成单一 `ProxyDecision`,
//! 再分别应用到 reqwest HTTP client 与 git 子进程环境变量。
//!
//! 设计为**纯函数**:不持有状态、不读库,调用方在构造 client / 执行 git 前
//! 临时读设置并调用。这样无需共享 client 单例,也无需 settings-changed 事件
//! 驱动重建——每次构造都拿到最新设置,天然生效。
//!
//! 账号级设置优先于全局:同一账号可能需要特定代理（如内网 GitLab 走专线）,
//! 不应被全局设置覆盖;账号未指定时才回退到全局兜底。

use crate::errors::{GitViewError, Result};
use crate::models::settings::NetworkSettings;

/// 代理决策结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProxyDecision {
    /// 跟随系统 / 进程环境代理（不显式设置,交由底层库读环境变量）
    System,
    /// 强制直连（显式禁用代理）
    None,
    /// 使用指定代理 URL
    Explicit(String),
}

/// 合并账号级与全局网络设置,得出最终代理决策。
///
/// 优先级（高 → 低）:
///   1. 账号显式代理 URL（非空）
///   2. 账号声明「跟随系统」
///   3. 全局显式代理（https 优先,其次 http）
///   4. 全局「跟随系统」
///   5. 默认直连
#[must_use]
pub fn resolve_proxy(
    net: &NetworkSettings,
    account_proxy: Option<&str>,
    account_use_system: bool,
) -> ProxyDecision {
    // 1. 账号显式代理优先级最高:账号可能需要专用通道,不容全局覆盖
    if let Some(url) = account_proxy.filter(|s| !s.is_empty()) {
        return ProxyDecision::Explicit(url.to_string());
    }
    // 2. 账号声明跟随系统
    if account_use_system {
        return ProxyDecision::System;
    }
    // 3. 全局显式代理:https 优先（API / clone 主要走 https）,回退 http
    if let Some(url) = net.https_proxy.as_deref().filter(|s| !s.is_empty()) {
        return ProxyDecision::Explicit(url.to_string());
    }
    if let Some(url) = net.http_proxy.as_deref().filter(|s| !s.is_empty()) {
        return ProxyDecision::Explicit(url.to_string());
    }
    // 4. 全局跟随系统
    if net.use_system_proxy {
        return ProxyDecision::System;
    }
    // 5. 都没配 → 直连
    ProxyDecision::None
}

/// 把代理决策应用到 reqwest `ClientBuilder`。
///
/// - System:不动 builder,reqwest 默认读 HTTP(S)_PROXY 环境变量,等于跟随系统
/// - None:显式 `.no_proxy()` 禁用,即便环境里有代理变量也直连
/// - Explicit:`Proxy::all` 让 http 与 https 都走该代理
pub fn apply_to_reqwest(
    builder: reqwest::ClientBuilder,
    decision: &ProxyDecision,
) -> Result<reqwest::ClientBuilder> {
    match decision {
        ProxyDecision::System => Ok(builder),
        ProxyDecision::None => Ok(builder.no_proxy()),
        ProxyDecision::Explicit(url) => {
            // 非法代理 URL 在此暴露,转成前端可理解的 ApiUrlInvalid
            let proxy = reqwest::Proxy::all(url)
                .map_err(|e| GitViewError::ApiUrlInvalid(format!("代理地址无效：{e}")))?;
            Ok(builder.proxy(proxy))
        }
    }
}

/// 把代理决策转成注入 git 子进程的环境变量。
///
/// 仅 Explicit 注入 `HTTP_PROXY`/`HTTPS_PROXY`;System/None 返回空:
/// git 子进程继承 GitView 进程环境,System 即沿用系统设置。
/// 注:None 若想强制清除已有环境代理需 `env_remove`,属少见需求,V1 不处理。
#[must_use]
pub fn git_proxy_env(decision: &ProxyDecision) -> Vec<(String, String)> {
    match decision {
        ProxyDecision::System | ProxyDecision::None => Vec::new(),
        ProxyDecision::Explicit(url) => vec![
            ("HTTP_PROXY".to_string(), url.clone()),
            ("HTTPS_PROXY".to_string(), url.clone()),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造一个只关心代理字段的 NetworkSettings。
    fn net(http: Option<&str>, https: Option<&str>, system: bool) -> NetworkSettings {
        NetworkSettings {
            http_proxy: http.map(str::to_string),
            https_proxy: https.map(str::to_string),
            use_system_proxy: system,
            ..NetworkSettings::default()
        }
    }

    /// 账号显式代理优先级最高,压过全局一切设置。
    #[test]
    fn account_explicit_wins_over_global() {
        let n = net(Some("http://global:8080"), None, true);
        let d = resolve_proxy(&n, Some("http://acct:3128"), true);
        assert_eq!(d, ProxyDecision::Explicit("http://acct:3128".into()));
    }

    /// 账号「跟随系统」压过全局显式代理。
    #[test]
    fn account_system_wins_over_global_explicit() {
        let n = net(Some("http://global:8080"), None, false);
        let d = resolve_proxy(&n, None, true);
        assert_eq!(d, ProxyDecision::System);
    }

    /// 账号未配时,全局 https 代理优先于 http。
    #[test]
    fn global_https_preferred_over_http() {
        let n = net(Some("http://h:1"), Some("http://s:2"), false);
        let d = resolve_proxy(&n, None, false);
        assert_eq!(d, ProxyDecision::Explicit("http://s:2".into()));
    }

    /// 账号与全局显式都没有,但全局跟随系统。
    #[test]
    fn falls_back_to_global_system() {
        let n = net(None, None, true);
        let d = resolve_proxy(&n, None, false);
        assert_eq!(d, ProxyDecision::System);
    }

    /// 全都没配 → 直连。
    #[test]
    fn defaults_to_none_when_nothing_set() {
        let n = net(None, None, false);
        let d = resolve_proxy(&n, None, false);
        assert_eq!(d, ProxyDecision::None);
    }

    /// 空字符串视为未配置,不当作 Explicit。
    #[test]
    fn empty_strings_are_ignored() {
        let n = net(Some(""), Some(""), false);
        let d = resolve_proxy(&n, Some(""), false);
        assert_eq!(d, ProxyDecision::None);
    }

    /// git env:仅 Explicit 注入 HTTP(S)_PROXY,System/None 为空。
    #[test]
    fn git_env_only_for_explicit() {
        assert!(git_proxy_env(&ProxyDecision::System).is_empty());
        assert!(git_proxy_env(&ProxyDecision::None).is_empty());
        let env = git_proxy_env(&ProxyDecision::Explicit("http://p:8080".into()));
        assert_eq!(env.len(), 2);
        assert!(env
            .iter()
            .any(|(k, v)| k == "HTTP_PROXY" && v == "http://p:8080"));
        assert!(env
            .iter()
            .any(|(k, v)| k == "HTTPS_PROXY" && v == "http://p:8080"));
    }

    /// apply_to_reqwest:三个分支都不应 panic,合法代理 URL 成功。
    #[test]
    fn apply_to_reqwest_covers_all_branches() {
        assert!(apply_to_reqwest(reqwest::Client::builder(), &ProxyDecision::System).is_ok());
        assert!(apply_to_reqwest(reqwest::Client::builder(), &ProxyDecision::None).is_ok());
        let r = apply_to_reqwest(
            reqwest::Client::builder(),
            &ProxyDecision::Explicit("http://p:8080".into()),
        );
        assert!(r.is_ok());
    }
}
