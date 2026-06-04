//! 应用设置领域模型。
//!
//! 设置在数据库 `settings` 表中以 key/value 形式存储（value 为 JSON 字符串）。
//! 本模块定义按**关注点分组**的设置结构,而非单一扁平大结构,原因:
//!   - 各组（通用 / Git / 网络 / 外部工具）以独立 key 存储,新增某组字段时
//!     只动该组,互不影响,也便于按组做强类型 get/set。
//!   - 前端设置页也按 Tab 分组,模型与 UI 结构对齐降低心智负担。
//!
//! key/value 设计使得新增设置项无需数据库迁移;读取端对缺失字段回退默认值,
//! 保证旧库平滑兼容（见 `settings_service::read_group`）。

use serde::{Deserialize, Serialize};

/// 默认克隆协议。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloneProtocol {
    /// HTTPS 克隆（兼容性最好,需要 token / 凭据）
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

/// 默认 pull 策略。
///
/// 默认 `FfOnly`：GUI 里悄悄产生 merge commit 是用户难以察觉的副作用,
/// 只允许快进可避免;分叉时明确报错交用户决定。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PullStrategy {
    /// `--ff-only`：仅允许快进合并
    FfOnly,
    /// `--rebase`：变基到远端之上
    Rebase,
    /// 普通合并（允许产生 merge commit）
    Merge,
}

/// 默认 push 策略（对应 git `push.default`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PushStrategy {
    /// simple：仅推送当前分支到同名上游（git 默认,最安全）
    Simple,
    /// current：推送当前分支到同名远端分支
    Current,
    /// upstream：推送到已配置的上游分支
    Upstream,
}

/// 通用设置组：目录、协议、并发、外观等高频项。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneralSettings {
    /// 默认仓库根目录（批量克隆时预填）
    pub default_repo_base_dir: String,
    /// 默认克隆协议
    pub default_clone_protocol: CloneProtocol,
    /// 默认并发克隆数（前端填入 clone payload;改动需重启生效）
    pub default_concurrency: u8,
    /// 目录组织策略
    pub directory_strategy: DirectoryStrategy,
    /// 界面主题
    pub theme: Theme,
    /// 界面语言
    pub language: Language,
    /// 启动时是否自动打开上次的仓库
    pub open_last_repo_on_startup: bool,
    /// 启动时是否自动检查本地仓库状态
    pub auto_check_repo_status: bool,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            // 默认放在用户主目录下的 Projects;取不到主目录时回退相对名
            default_repo_base_dir: default_repo_base_dir(),
            default_clone_protocol: CloneProtocol::Https,
            // 3 个并发是经验上兼顾速度与服务端压力的折中默认
            default_concurrency: 3,
            // 多平台多账号场景下两级分组最不易冲突,作默认
            directory_strategy: DirectoryStrategy::ByPlatformAndOwner,
            theme: Theme::Auto,
            language: Language::ZhCn,
            open_last_repo_on_startup: false,
            // 默认开启:打开应用即能看到各仓库是否有未提交/落后
            auto_check_repo_status: true,
        }
    }
}

/// Git 设置组：可执行路径与提交身份、默认网络策略。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitSettings {
    /// 自定义 git 可执行路径（None 表示用 PATH 自动探测）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_executable_path: Option<String>,
    /// 提交身份 user.name（None 表示沿用 git 全局配置）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_name: Option<String>,
    /// 提交身份 user.email（None 表示沿用 git 全局配置）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_email: Option<String>,
    /// 默认 pull 策略
    pub default_pull_strategy: PullStrategy,
    /// 默认 push 策略
    pub default_push_strategy: PushStrategy,
}

impl Default for GitSettings {
    fn default() -> Self {
        Self {
            git_executable_path: None,
            user_name: None,
            user_email: None,
            // 与 git_cli_service 的 pull 实现（--ff-only）保持一致
            default_pull_strategy: PullStrategy::FfOnly,
            default_push_strategy: PushStrategy::Simple,
        }
    }
}

/// 网络设置组：代理与超时。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkSettings {
    /// HTTP 代理 URL（None 表示不显式设置）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_proxy: Option<String>,
    /// HTTPS 代理 URL（None 表示不显式设置）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub https_proxy: Option<String>,
    /// 是否跟随系统代理（开启时忽略上面两项,让底层库读系统设置）
    pub use_system_proxy: bool,
    /// API 请求超时（秒）
    pub api_timeout_secs: u32,
    /// 克隆超时（秒）
    pub clone_timeout_secs: u32,
}

impl Default for NetworkSettings {
    fn default() -> Self {
        Self {
            http_proxy: None,
            https_proxy: None,
            // 默认不强制系统代理:多数用户直连,需要时再显式开启
            use_system_proxy: false,
            // 30s 足够普通 API 往返,又不会让故障长时间挂起
            api_timeout_secs: 30,
            // 克隆大仓库耗时较长,给 5 分钟上限
            clone_timeout_secs: 300,
        }
    }
}

/// 外部工具设置组：用「在外部工具打开」时调用的命令。
///
/// 字段统一以 `_command` 结尾是有意的:存的是可执行命令字符串（如 `code`）,
/// 后缀明确「值是命令而非工具对象」,语义清晰;故局部豁免 struct_field_names。
#[allow(clippy::struct_field_names)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalToolsSettings {
    /// 默认编辑器命令（如 `code` / `cursor`,None 表示未配置）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub editor_command: Option<String>,
    /// 默认终端命令（None 表示未配置）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_command: Option<String>,
    /// 默认文件管理器命令（None 表示未配置）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_manager_command: Option<String>,
}

/// 应用设置聚合快照（service 层组装四组后返回前端的完整设置）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// 通用组
    pub general: GeneralSettings,
    /// Git 组
    pub git: GitSettings,
    /// 网络组
    pub network: NetworkSettings,
    /// 外部工具组
    pub external_tools: ExternalToolsSettings,
}

/// Git 环境检测结果（detect_git / set_git_path 返回给前端）。
///
/// 用 `found` 而非直接报错表达「未检测到 git」：未装 git 是正常可处理状态,
/// 前端据此引导用户安装或手动指定路径,而不是当成异常抛红。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitDetectionResult {
    /// 是否检测到可用的 git
    pub found: bool,
    /// git 可执行路径
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// `git --version` 解析出的版本号
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// 读取到的 user.name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_name: Option<String>,
    /// 读取到的 user.email
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_email: Option<String>,
}

/// 计算默认仓库根目录。
///
/// 跨平台取用户主目录:Unix 用 `HOME`,Windows 用 `USERPROFILE`;
/// 都取不到时回退相对名 `Projects`（注释提醒:此兜底极少触发,
/// 真实环境基本都有主目录,前端也可让用户再行选择）。
fn default_repo_base_dir() -> String {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_default();
    if home.is_empty() {
        "Projects".to_string()
    } else {
        format!("{home}/Projects")
    }
}
