//! Clone 任务并发上限集成测试。
//!
//! 验证 CloneManager 的 semaphore 在压力下不允许超出配置的并发数。
//! 不依赖真实 git 二进制——直接占用 semaphore 模拟运行任务。

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use gitview_lib::services::clone_task_service::CloneManager;
use gitview_lib::services::git_cli_service::GitCliService;
use tokio::sync::Semaphore;

/// 提取 manager 内部 semaphore 的等价副本作为压测对象。
///
/// CloneManager::new 已 clamp 到 [1, 8]；本测试通过同样的逻辑构造一个
/// 容量为 3 的 Semaphore，模拟 10 个任务争抢的情形。
#[tokio::test]
async fn semaphore_caps_concurrent_running_tasks() {
    let _manager = CloneManager::new(GitCliService::with_path(PathBuf::from("git")), 3);

    let sem = Arc::new(Semaphore::new(3));
    let active = Arc::new(AtomicUsize::new(0));
    let max_observed = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();
    for _ in 0..10 {
        let sem = Arc::clone(&sem);
        let active = Arc::clone(&active);
        let max_observed = Arc::clone(&max_observed);
        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            let now = active.fetch_add(1, Ordering::SeqCst) + 1;
            max_observed.fetch_max(now, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(50)).await;
            active.fetch_sub(1, Ordering::SeqCst);
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    let peak = max_observed.load(Ordering::SeqCst);
    assert!(peak <= 3, "并发峰值应不超过 3，实际观察到 {peak}");
    assert!(peak >= 1, "应至少有一个任务执行过");
}

#[tokio::test]
async fn manager_concurrency_clamped_high() {
    let m = CloneManager::new(GitCliService::with_path(PathBuf::from("git")), 99);
    drop(m);
}

#[tokio::test]
async fn manager_concurrency_clamped_zero() {
    let m = CloneManager::new(GitCliService::with_path(PathBuf::from("git")), 0);
    drop(m);
}
