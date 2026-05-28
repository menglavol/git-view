//! 应用设置领域模型。
//!
//! 设置在数据库 `settings` 表中以 key/value 形式存储，本模块定义：
//!   - `Settings` 聚合结构：service 层组装后返回给前端的完整设置快照
//!   - 各类设置枚举：克隆协议、目录策略、主题、语言等
//!
//! 数据库层面采用 key/value 而非固定列，便于后续新增设置项无需迁移。

use serde::{Deserialize, Serialize};

/// 默认克隆协议。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloneProtocol {
    /// HTTPS 克隆（兼容性最好，需要 token / 凭据）
    Https,
    /// SSH 克隆（需要本地 SSH key 配置）
    Ssh,
}

/// 仓库目录组织策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectoryStrategy {
    /// 扁平：`<base>/<repo_name>`
    Flat,
    /// 按所有者分组：`<base>/<owner>/<repo_name>`
    ByOwner,
    /// 按平台与所有者两级分组：`<base>/<platform>/<owner>/<repo_name>`
    ByPlatformAndOwner,
}

/// 界面主题。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    /// 跟随系统
    Auto,
    /// 浅色
    Light,
    /// 深色
    Dark,
}

/// 界面语言。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    /// 简体中文
    ZhCn,
    /// English
    EnUs,
}

/// 应用设置聚合（service 层组装后返回前端的完整快照）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// 默认仓库根目录
    pub default_repo_base_dir: String,
    /// 默认克隆协议
    pub default_clone_protocol: CloneProtocol,
    /// 目录组织策略
    pub directory_strategy: DirectoryStrategy,
    /// 界面主题
    pub theme: Theme,
    /// 界面语言
    pub language: Language,
    /// 自定义 git 可执行路径（可空：使用 PATH 查找）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_executable_path: Option<String>,
    /// HTTP 代理 URL（可空）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_proxy: Option<String>,
    /// HTTPS 代理 URL（可空）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub https_proxy: Option<String>,
    /// 是否启用启动时自动同步
    pub auto_sync_on_startup: bool,
}
