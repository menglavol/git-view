//! 本地仓库扫描集成测试（T075 / US4）。
//!
//! 验证 `repository_service::scan_local_repositories` 的关键不变量：
//!   1. 父目录下的多个 Git 仓库能被识别并入库（基础识别）
//!   2. 非 Git 目录、与 `.git` 同名但非 Git 元数据的目录均被忽略
//!   3. 二次扫描同一目录不会重复添加（去重幂等）
//!
//! 实现说明：
//!   - 用 `tempfile::TempDir` 隔离测试目录，结束自动清理
//!   - 用 `git init -q` 真实初始化仓库；这是最稳的桩，避免手工伪造 `.git` 结构
//!   - 用内存 SQLite pool（NamedTempFile 持有临时文件路径）与 `repository_sync_test.rs`
//!     保持一致
//!
//! 该测试依赖系统 PATH 中存在 `git` 可执行文件；CI 上需确保安装。

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;
use std::process::Command;

use gitview_lib::db::migrations::run_pending_migrations;
use gitview_lib::db::pool::DbPool;
use gitview_lib::services::repository_service;

/// 初始化一个干净的内存 SQLite pool 并跑完迁移。
fn fresh_pool() -> DbPool {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    // 让 NamedTempFile 释放文件句柄但保留磁盘文件，便于 rusqlite 自行打开
    let _ = tmp.keep();
    let pool = DbPool::new(&path).unwrap();
    run_pending_migrations(&pool).unwrap();
    pool
}

/// 在指定目录运行 `git init -q`；失败时 panic（测试前置依赖）。
fn git_init(repo_dir: &Path) {
    std::fs::create_dir_all(repo_dir).unwrap();
    let status = Command::new("git")
        .arg("init")
        .arg("-q")
        .arg(repo_dir)
        .status()
        .expect("git 未安装，无法运行本地仓库扫描测试");
    assert!(status.success(), "git init 失败：{}", repo_dir.display());
}

/// 验证扫描能识别多个仓库、忽略非仓库目录，并对二次扫描幂等。
#[tokio::test]
#[ignore = "依赖系统 git CLI 与文件系统，按 cargo test -- --ignored 手动跑"]
async fn scan_identifies_and_dedupes_repositories() {
    let root = tempfile::tempdir().unwrap();
    let root_path = root.path();

    // 准备 3 个真实 Git 仓库
    let repo_a = root_path.join("proj-a");
    let repo_b = root_path.join("proj-b");
    let repo_c = root_path.join("group").join("proj-c");
    git_init(&repo_a);
    git_init(&repo_b);
    git_init(&repo_c);

    // 干扰目录 1：纯普通目录（不是 Git 仓库）
    std::fs::create_dir_all(root_path.join("not-a-repo").join("src")).unwrap();
    // 干扰目录 2：名字像 .git 的伪装目录（无 HEAD 文件，不会被识别）
    std::fs::create_dir_all(root_path.join("decoy").join(".git-like")).unwrap();

    let pool = fresh_pool();

    // 第一次扫描：应识别出 3 个仓库（scan 返回 ScanResult，新增列表在 added 字段）
    let first = repository_service::scan_local_repositories(&pool, root_path, 5)
        .await
        .unwrap();
    assert_eq!(
        first.added.len(),
        3,
        "首次扫描应识别 3 个仓库，实际：{first:?}"
    );

    // 列表查询同样应得到 3 条
    let listed = repository_service::list_local_repositories(&pool).unwrap();
    assert_eq!(listed.len(), 3);

    // 第二次扫描：去重生效，不再新增
    let second = repository_service::scan_local_repositories(&pool, root_path, 5)
        .await
        .unwrap();
    // V1 实现：scan_local_repositories 内部对每个发现的目录都会调 add_local_repository，
    // 而后者对已存在路径会返回旧记录而不是新插入。返回值长度可能仍为 3（同一组旧记录），
    // 关键不变量是 DB 中仓库总数不增加。
    assert_eq!(
        repository_service::list_local_repositories(&pool)
            .unwrap()
            .len(),
        3,
        "二次扫描后 DB 总数应仍为 3，实际新增：{second:?}"
    );
}
