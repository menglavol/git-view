//! 设置持久化集成测试（T106 / US7）。
//!
//! 验证 `settings_service` 写入的设置能**跨连接重开**保持——即真正落盘到 SQLite，
//! 而不是只活在内存连接里。这正对应 US7 验收标准「修改各项设置后重启应用，设置
//! 保持生效」：重启 = 进程退出后重新打开同一个 DB 文件，本测试用「drop 旧 DbPool
//! → 新建一个指向同一路径的 DbPool」来模拟那次「关闭再打开」。
//!
//! 实现说明：
//!   - 用 `tempfile::tempdir()` 下的固定文件名，保证两个 DbPool 打开的是同一物理文件；
//!   - 第一个 pool 在作用域结束时 drop，释放 SQLite 连接，再开第二个 pool 读取；
//!   - 仅依赖 SQLite（无 git CLI、无网络），可直接进 CI，无需 `--ignored`。

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use gitview_lib::db::migrations::run_pending_migrations;
use gitview_lib::db::pool::DbPool;
use gitview_lib::models::settings::{
    CloneProtocol, DirectoryStrategy, ExternalToolsSettings, GeneralSettings, GitSettings,
    Language, NetworkSettings, PullStrategy, PushStrategy, Settings, Theme,
};
use gitview_lib::services::settings_service;

/// 在给定文件路径上建一个已迁移的 DbPool（settings 表由 001_init 建出）。
fn open_pool(db_path: &Path) -> DbPool {
    // 文件不存在时 SQLite 会自动创建（父目录由调用方的 tempdir 保证存在）
    let pool = DbPool::new(db_path).unwrap();
    run_pending_migrations(&pool).unwrap();
    pool
}

/// 构造一组「明显非默认」的设置。
///
/// 故意让每个字段都偏离 `Default`，这样第二阶段读出的值若与之相等，
/// 就能确证读到的是**写入并落盘的值**，而非宽容回退的默认值（排除假阳性）。
fn sample_settings() -> Settings {
    Settings {
        // 通用组：6 个字段改成非默认值，其余两个布尔沿用默认
        general: GeneralSettings {
            default_repo_base_dir: "/tmp/gitview-test-repos".to_string(),
            default_clone_protocol: CloneProtocol::Ssh,
            default_concurrency: 7,
            directory_strategy: DirectoryStrategy::Flat,
            theme: Theme::Dark,
            language: Language::EnUs,
            ..GeneralSettings::default()
        },
        // Git 组：路径与身份都填上，pull/push 改成非默认策略
        git: GitSettings {
            git_executable_path: Some("/usr/local/bin/git".to_string()),
            user_name: Some("Persisted User".to_string()),
            user_email: Some("persist@example.com".to_string()),
            default_pull_strategy: PullStrategy::Rebase,
            default_push_strategy: PushStrategy::Upstream,
        },
        // 网络组：开系统代理 + 自定义超时；https_proxy 留默认 None
        network: NetworkSettings {
            http_proxy: Some("http://127.0.0.1:7890".to_string()),
            use_system_proxy: true,
            api_timeout_secs: 45,
            clone_timeout_secs: 900,
            ..NetworkSettings::default()
        },
        // 外部工具组：只填编辑器命令
        external_tools: ExternalToolsSettings {
            editor_command: Some("code".to_string()),
            ..ExternalToolsSettings::default()
        },
    }
}

/// 验收：完整设置写入后，重开连接仍能逐字段读回（跨「重启」持久化）。
#[test]
fn settings_persist_across_pool_reopen() {
    let dir = tempfile::tempdir().unwrap(); // 测试结束时整目录清理
    let db_path = dir.path().join("gitview-settings.db"); // 两个 pool 共用此文件
    let expected = sample_settings();

    // —— 第一阶段：写入并在同一连接内确认写成功 ——
    {
        let pool = open_pool(&db_path);
        settings_service::update_settings(&pool, &expected).unwrap(); // 原子写四组
        let in_session = settings_service::get_settings(&pool).unwrap();
        assert_eq!(in_session.general.default_concurrency, 7); // 同连接内先确认写入生效
                                                               // pool 在此 drop，释放 SQLite 连接，模拟应用退出
    }

    // —— 第二阶段：重开指向同一文件的新连接，验证设置仍在 ——
    let pool2 = open_pool(&db_path);
    let reopened = settings_service::get_settings(&pool2).unwrap();

    // 通用组：逐字段比对
    assert_eq!(
        reopened.general.default_repo_base_dir,
        "/tmp/gitview-test-repos"
    );
    assert!(matches!(
        reopened.general.default_clone_protocol,
        CloneProtocol::Ssh
    ));
    assert_eq!(reopened.general.default_concurrency, 7); // 并发数保持
    assert!(matches!(
        reopened.general.directory_strategy,
        DirectoryStrategy::Flat
    ));
    assert!(matches!(reopened.general.theme, Theme::Dark)); // 深色主题保持
    assert!(matches!(reopened.general.language, Language::EnUs)); // 英文语言保持

    // Git 组：路径、身份、pull/push 策略
    assert_eq!(
        reopened.git.git_executable_path.as_deref(),
        Some("/usr/local/bin/git")
    );
    assert_eq!(reopened.git.user_name.as_deref(), Some("Persisted User"));
    assert_eq!(
        reopened.git.user_email.as_deref(),
        Some("persist@example.com")
    );
    assert!(matches!(
        reopened.git.default_pull_strategy,
        PullStrategy::Rebase
    ));
    assert!(matches!(
        reopened.git.default_push_strategy,
        PushStrategy::Upstream
    ));

    // 网络组：代理与超时
    assert_eq!(
        reopened.network.http_proxy.as_deref(),
        Some("http://127.0.0.1:7890")
    );
    assert!(reopened.network.use_system_proxy); // 系统代理开关跨重开保持
    assert_eq!(reopened.network.api_timeout_secs, 45); // API 超时保持
    assert_eq!(reopened.network.clone_timeout_secs, 900); // 克隆超时保持

    // 外部工具组：编辑器命令
    assert_eq!(
        reopened.external_tools.editor_command.as_deref(),
        Some("code")
    );
}

/// 验收：只写一组时，该组跨重开保持，未写入的组读出默认值（宽容回退）。
#[test]
fn partial_group_set_persists_others_default() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("gitview-partial.db");

    // 第一阶段：仅写网络组（api 超时改成 123）
    {
        let pool = open_pool(&db_path);
        let net = NetworkSettings {
            api_timeout_secs: 123,
            ..NetworkSettings::default()
        };
        settings_service::set_network(&pool, &net).unwrap();
    }

    // 第二阶段：重开后，网络组保持、通用组回退默认
    let pool2 = open_pool(&db_path);
    assert_eq!(
        settings_service::get_network(&pool2)
            .unwrap()
            .api_timeout_secs,
        123,
        "写入的网络组应跨重开保持"
    );
    assert_eq!(
        settings_service::get_general(&pool2)
            .unwrap()
            .default_concurrency,
        3,
        "未写入的通用组应回退到默认并发 3"
    );
}
