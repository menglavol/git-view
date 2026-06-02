//! 单仓库 Git 工作流集成测试（T090 / T091）。
//!
//! 覆盖：
//! - T090：commit 通过临时文件 `.git/COMMIT_GITVIEW` 提交时,多行 message 与
//!   中文字符能正确入库,且临时文件在提交后被清理
//! - T091：`pre_commit_check` 5 项校验逐一覆盖,任一未通过时返回明确的中文
//!   Internal 错误
//!
//! 依赖：
//!   - 系统 PATH 中存在 `git` 可执行文件
//!   - `tempfile::tempdir` 隔离测试仓库,结束自动清理
//!
//! CI 策略：与 `repository_scan_test.rs` 一致,标 `#[ignore]` 由
//!         `cargo test -- --ignored` 手动触发。

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::process::Command;

use gitview_lib::errors::GitViewError;
use gitview_lib::services::git_cli_service::GitCliService;

// =====================================================================
// 测试辅助:在临时目录初始化一个完整的 Git 仓库
// =====================================================================

/// 初始化一个临时 Git 仓库,并配置 user.name / user.email 让 commit 不被阻断。
fn init_repo_with_identity() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("无法创建临时目录");
    let path = tmp.path();
    run_git(path, &["init", "-q"]);
    // 测试身份:避免影响用户的全局 git config
    run_git(path, &["config", "user.name", "GitView 测试"]);
    run_git(path, &["config", "user.email", "test@gitview.local"]);
    // 默认分支名固定为 main,避免不同 Git 版本默认 master / main 差异
    run_git(path, &["checkout", "-q", "-b", "main"]);
    tmp
}

/// 在指定目录运行 git 子命令,失败时 panic 并打印 stderr。
fn run_git(cwd: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("LC_ALL", "C")
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .expect("git 未安装,无法运行 git_workflow_test");
    assert!(
        output.status.success(),
        "git {} 失败: stderr={}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr),
    );
}

/// 写入文件并把它 stage,作为 commit 前置准备。
fn create_and_stage(repo: &Path, file: &str, content: &str) {
    let target = repo.join(file);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&target, content).unwrap();
    run_git(repo, &["add", "--", file]);
}

/// 取出当前仓库 HEAD 的完整 commit message(含正文)。
fn head_commit_message(repo: &Path) -> String {
    let output = Command::new("git")
        .args(["log", "-1", "--pretty=format:%B"])
        .current_dir(repo)
        .env("LC_ALL", "C")
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// 构造 GitCliService 实例(PATH 中的 git)。
fn make_cli() -> GitCliService {
    GitCliService::with_path(PathBuf::from("git"))
}

// =====================================================================
// T090 — commit 临时文件提交测试
// =====================================================================

/// 验收(T090):多行 message + description 通过临时文件正确入库,且临时文件
/// 在提交完成后被清理。
#[tokio::test]
#[ignore = "依赖系统 git CLI 与文件系统,按 cargo test -- --ignored 手动跑"]
async fn commit_writes_multiline_and_cleans_up_temp_file() {
    let tmp = init_repo_with_identity();
    let repo = tmp.path();

    create_and_stage(repo, "README.md", "hello\n");

    let cli = make_cli();
    let message = "feat: 添加 README";
    let description = "支持中文与多行描述\n\n第二段说明:\n - 列表项 1\n - 列表项 2";

    let stdout = cli.commit(repo, message, Some(description)).await.unwrap();
    assert!(
        stdout.contains("main") || stdout.to_lowercase().contains("commit"),
        "commit stdout 应包含分支或 commit 标识:{stdout}"
    );

    // 验证 HEAD message 包含原 message 与 description
    let body = head_commit_message(repo);
    assert!(body.contains(message), "HEAD message 应含标题: {body}");
    assert!(
        body.contains("支持中文与多行描述"),
        "HEAD message 应含中文 description: {body}"
    );
    assert!(body.contains("列表项 1"), "HEAD message 应保留多行结构");

    // 临时文件必须被清理(无论成功失败)
    let temp_file = repo.join(".git").join("COMMIT_GITVIEW");
    assert!(
        !temp_file.exists(),
        "COMMIT_GITVIEW 临时文件应在 commit 后被清理: {temp_file:?}"
    );
}

/// 验收(T090):特殊字符 / 引号 / 反斜杠 / 换行均不被命令行转义破坏。
#[tokio::test]
#[ignore = "依赖系统 git CLI 与文件系统,按 cargo test -- --ignored 手动跑"]
async fn commit_preserves_special_chars() {
    let tmp = init_repo_with_identity();
    let repo = tmp.path();

    create_and_stage(repo, "x.txt", "x\n");

    let cli = make_cli();
    // 含双引号、单引号、反斜杠、$、`、换行、中文标点
    let message = r#"修复:"路径解析"问题(含 \\转义、'引号'、$VAR、`code`)"#;
    cli.commit(repo, message, None).await.unwrap();

    let body = head_commit_message(repo);
    assert!(body.contains("路径解析"), "应保留中文引号内容: {body}");
    assert!(body.contains(r"\\转义"), "应保留反斜杠: {body}");
    assert!(
        body.contains("$VAR"),
        "应保留美元符号未被 shell 展开: {body}"
    );
}

// =====================================================================
// T091 — commit 前置 5 项校验测试
// =====================================================================

/// 验收(T091.1):git user.name / user.email 未配置时返回 Internal 含中文
/// 阻断原因。
///
/// 注:`git config --get` 会回退到全局 ~/.gitconfig,因此本测试通过
/// `GIT_CONFIG_NOSYSTEM=1` + `GIT_CONFIG_GLOBAL=<空文件>` 屏蔽全局配置,
/// 避免用户本地 git 设置干扰断言。
#[tokio::test]
#[ignore = "依赖系统 git CLI 与文件系统,按 cargo test -- --ignored 手动跑"]
async fn pre_commit_check_blocks_when_git_identity_missing() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();

    // 屏蔽全局 git 配置(只在本测试进程内生效)
    let empty_global = tmp.path().join(".empty-gitconfig");
    std::fs::write(&empty_global, "").unwrap();
    // SAFETY: 这两个 env var 设置在测试主线程,作用域仅本测试函数;
    // 因为 #[tokio::test] 串行执行 + 进程退出即清理,影响范围受限。
    std::env::set_var("GIT_CONFIG_NOSYSTEM", "1");
    std::env::set_var("GIT_CONFIG_GLOBAL", &empty_global);

    // 初始化仓库,故意不配置 user.name / user.email
    let output = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(repo)
        .env("LC_ALL", "C")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", &empty_global)
        .status()
        .unwrap();
    assert!(output.success());

    let cli = make_cli();
    let err = cli.pre_commit_check(repo).await.unwrap_err();
    match err {
        GitViewError::Internal(msg) => {
            assert!(
                msg.contains("user.name") || msg.contains("user.email"),
                "错误信息应提到 user.name 或 user.email: {msg}"
            );
        }
        other => panic!("应返回 Internal 错误,实际:{other:?}"),
    }

    // 清理 env var,避免影响后续测试
    std::env::remove_var("GIT_CONFIG_NOSYSTEM");
    std::env::remove_var("GIT_CONFIG_GLOBAL");
}

/// 验收(T091.3):无任何已暂存文件时返回 Internal 阻断。
#[tokio::test]
#[ignore = "依赖系统 git CLI 与文件系统,按 cargo test -- --ignored 手动跑"]
async fn pre_commit_check_blocks_when_no_staged_files() {
    let tmp = init_repo_with_identity();
    let repo = tmp.path();

    // 写入一个文件但不 stage(只有工作区变更)
    std::fs::write(repo.join("untouched.txt"), "x\n").unwrap();

    let cli = make_cli();
    let err = cli.pre_commit_check(repo).await.unwrap_err();
    match err {
        GitViewError::Internal(msg) => {
            assert!(msg.contains("暂存"), "错误信息应提到暂存: {msg}");
        }
        other => panic!("应返回 Internal,实际:{other:?}"),
    }
}

/// 验收(T091.4):5 项全部满足时 pre_commit_check 通过(Ok 路径)。
#[tokio::test]
#[ignore = "依赖系统 git CLI 与文件系统,按 cargo test -- --ignored 手动跑"]
async fn pre_commit_check_passes_when_all_satisfied() {
    let tmp = init_repo_with_identity();
    let repo = tmp.path();

    create_and_stage(repo, "ok.txt", "ready to commit\n");

    let cli = make_cli();
    cli.pre_commit_check(repo)
        .await
        .expect("5 项都满足时应通过");
}

/// 验收(T091.5):detached HEAD 状态下返回 Internal 阻断。
///
/// 通过先完成一次 commit、再 `git checkout <sha>` 切到 detached HEAD 来构造。
#[tokio::test]
#[ignore = "依赖系统 git CLI 与文件系统,按 cargo test -- --ignored 手动跑"]
async fn pre_commit_check_blocks_when_detached_head() {
    let tmp = init_repo_with_identity();
    let repo = tmp.path();

    // 先完成一次正常 commit,得到一个 sha 可供 detach
    create_and_stage(repo, "init.txt", "x\n");
    run_git(repo, &["commit", "-q", "-m", "init"]);

    // 取得 HEAD 的完整 sha 后 detach 过去
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo)
        .output()
        .unwrap();
    let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
    run_git(repo, &["checkout", "-q", &sha]);

    // 在 detached HEAD 下 stage 一个文件
    create_and_stage(repo, "x2.txt", "y\n");

    let cli = make_cli();
    let err = cli.pre_commit_check(repo).await.unwrap_err();
    match err {
        GitViewError::Internal(msg) => {
            assert!(
                msg.contains("detached"),
                "错误信息应提到 detached HEAD: {msg}"
            );
        }
        other => panic!("应返回 Internal,实际:{other:?}"),
    }
}
