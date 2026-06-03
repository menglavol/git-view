//! 操作日志脱敏端到端集成测试（T098 / US6）。
//!
//! 验证宪法 Principle III 与 spec SC-009 / SC-010：
//!   - 带 token 的操作经 `log_service::record_operation` 写入 `operation_logs` 后，
//!     库内任何文本列都不残留 token 明文（GitHub PAT / GitLab PAT / Bearer / URL 内联凭据）
//!   - URL 内联凭据被替换为 `<REDACTED>@host`，独立 token 被替换为 `<REDACTED-TOKEN>`
//!   - `list_operations` 返回的结构化数据同样脱敏，且失败日志带中文错误翻译
//!
//! 本文件从 crate 外部（`gitview_lib::`）通过公共 API 走真实落库路径，
//! 与 `log_service` 内部 #[cfg(test)] 单元测试互补，聚焦端到端可观测契约。

#![allow(clippy::unwrap_used, clippy::expect_used)]

use gitview_lib::db::migrations::run_pending_migrations;
use gitview_lib::db::pool::DbPool;
use gitview_lib::models::operation_log::{LogFilter, OperationStatus, OperationType};
use gitview_lib::services::log_service;

// 伪造 token 样本（仅测试用，非真实凭据）。
const GH_TOKEN: &str = "ghp_AbCdEfGhIjKlMnOpQrStUvWxYz0123456789ABCD"; // allow-token-pattern: 测试样本
const GL_TOKEN: &str = "glpat-AbCdEfGhIj1234567890"; // allow-token-pattern: 测试样本

/// 建一个已完成迁移的临时数据库。
///
/// 返回的 `NamedTempFile` 必须由调用方持有，否则文件在函数返回后被删除。
/// 每个用例独享一个临时库，保证测试相互隔离、可并行，不污染开发库。
fn make_db() -> (DbPool, tempfile::NamedTempFile) {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let pool = DbPool::new(tmp.path()).unwrap();
    // 跑全部迁移：脱敏断言依赖 003_extend_operation_logs 建出的 output/error 列
    run_pending_migrations(&pool).unwrap();
    (pool, tmp)
}

/// 把 `operation_logs` 全表所有文本列拼成一个大字符串，用于子串断言。
///
/// 直接读原始列（不经 service 反序列化），确保断言的是「库内真实落地的字节」，
/// 而不是 service 在返回前可能二次处理过的结果——脱敏必须发生在写入时。
fn dump_all_text(pool: &DbPool) -> String {
    pool.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, operation_type, target, status,
                    IFNULL(command, ''), IFNULL(output, ''), IFNULL(error_message, '')
             FROM operation_logs",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(format!(
                "{}|{}|{}|{}|{}|{}|{}",
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
            ))
        })?;
        let mut all = String::new();
        for r in rows {
            all.push_str(&r?);
            all.push('\n');
        }
        Ok(all)
    })
    .unwrap()
}

/// 验收：写入多种带 token 的操作后，库内不残留任何 token 明文。
///
/// 覆盖 4 种凭据形态：URL 内联、GitHub PAT、GitLab PAT、Bearer 头，
/// 确保脱敏正则不会漏掉其中任意一种。
#[test]
fn operation_logs_never_persist_token_bytes() {
    let (pool, _tmp) = make_db();

    // 模拟 push 失败：命令含 URL 内联 token，错误含独立 GitHub PAT
    log_service::record_operation(
        &pool,
        OperationType::Push,
        "myorg/myrepo",
        OperationStatus::Failed,
        Some(&format!(
            "git push https://oauth2:{GH_TOKEN}@github.com/myorg/myrepo.git"
        )),
        None,
        Some(&format!("remote: Authentication failed for {GH_TOKEN}")),
        100,
    )
    .unwrap();

    // 模拟 test_connection 失败：输出含 Bearer 头，错误含独立 GitLab PAT
    log_service::record_operation(
        &pool,
        OperationType::TestConnection,
        "gitlab-self-hosted",
        OperationStatus::Failed,
        None,
        Some(&format!("Authorization: Bearer {GL_TOKEN}")),
        Some(&format!("401 Unauthorized (token={GL_TOKEN})")),
        50,
    )
    .unwrap();

    // 两条日志均已落库
    let logs = log_service::list_operations(&pool, &LogFilter::default()).unwrap();
    assert_eq!(logs.len(), 2, "两条日志应均已写入");

    let dump = dump_all_text(&pool);
    // 下面用子串断言覆盖两种泄漏形态：完整 token 与仅前缀。
    // 任何 token 字节都不得残留——前缀检查能抓住「截断后仍泄漏」的回归
    assert!(!dump.contains(GH_TOKEN), "GitHub token 残留于库内");
    assert!(!dump.contains(GL_TOKEN), "GitLab token 残留于库内");
    assert!(!dump.contains("ghp_"), "GitHub PAT 前缀残留于库内");
    assert!(!dump.contains("glpat-"), "GitLab PAT 前缀残留于库内");
    // URL 内联凭据应被替换为 <REDACTED>@host
    assert!(dump.contains("<REDACTED>@github.com"), "URL 凭据未脱敏");
    // 独立 token 应被替换为占位符，证明写入路径确实执行了脱敏
    assert!(dump.contains("<REDACTED-TOKEN>"), "未见脱敏占位符");
}

/// 验收：list_operations 返回的结构化字段同样脱敏，且认证失败带中文翻译。
///
/// 与上一个用例互补：那个查库内原始字节，这个查 service 返回的结构化对象，
/// 双向确认「无论从哪条路径读都拿不到明文 token」。
#[test]
fn list_operations_returns_scrubbed_and_translated() {
    let (pool, _tmp) = make_db();
    // 命令里塞 URL 内联 token，错误信息用真实的 git 认证失败文案触发翻译
    log_service::record_operation(
        &pool,
        OperationType::Push,
        "repo",
        OperationStatus::Failed,
        Some(&format!("git push https://{GH_TOKEN}@github.com/x.git")),
        None,
        Some("fatal: Authentication failed for 'https://github.com/x.git'"),
        80,
    )
    .unwrap();

    let logs = log_service::list_operations(&pool, &LogFilter::default()).unwrap();
    assert_eq!(logs.len(), 1);
    let log = &logs[0];
    // 结构化 command 字段不得残留 token 前缀
    assert!(
        !log.command.as_ref().unwrap().contains("ghp_"), // allow-token-pattern: 断言无 token
        "command 字段残留 token"
    );
    // "Authentication failed" → 应命中错误翻译表，给出中文提示。
    // 这验证脱敏与翻译是两条独立链路：脱敏后仍能正确识别错误类型并翻译
    let translated = log
        .translated_error_message
        .as_ref()
        .expect("认证失败应有中文翻译");
    assert!(translated.contains("认证失败"), "翻译文案不符合预期");
}
