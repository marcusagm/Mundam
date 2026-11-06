use super::*;
use std::time::Duration;

async fn wait_for_queue(
    budget: &ProcessingBudget,
    count: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::time::timeout(Duration::from_secs(2), async {
        while budget.snapshot().waiting != count {
            tokio::task::yield_now().await;
        }
    })
    .await?;

    Ok(())
}

fn request(
    budget: &Arc<ProcessingBudget>,
    worker: WorkerKind,
    token: &CancellationToken,
) -> tokio::task::JoinHandle<Option<ResourcePermit>> {
    let budget = budget.clone();
    let token = token.clone();
    tokio::spawn(async move { budget.acquire(worker, &token).await })
}

#[tokio::test]
async fn processing_budget_shrinks_by_draining_and_grows_without_restart(
) -> Result<(), Box<dyn std::error::Error>> {
    let budget = Arc::new(ProcessingBudget::new(8));
    budget.configure(3, 3);
    let token = CancellationToken::new();
    let first = budget.acquire(WorkerKind::Thumbnails, &token).await.ok_or(
        "Expected a value in processing_budget_shrinks_by_draining_and_grows_without_restart",
    )?;
    let second = budget.acquire(WorkerKind::Thumbnails, &token).await.ok_or(
        "Expected a value in processing_budget_shrinks_by_draining_and_grows_without_restart",
    )?;
    let third = budget.acquire(WorkerKind::Colors, &token).await.ok_or(
        "Expected a value in processing_budget_shrinks_by_draining_and_grows_without_restart",
    )?;
    budget.configure(1, 1);
    let waiting = request(&budget, WorkerKind::Duplicates, &token);
    wait_for_queue(&budget, 1).await?;
    drop(first);
    drop(second);
    assert_eq!(budget.snapshot().active, 1);
    assert!(!waiting.is_finished());
    budget.configure(2, 1);
    let admitted = tokio::time::timeout(Duration::from_secs(2), waiting)
        .await??
        .ok_or(
            "Expected a value in processing_budget_shrinks_by_draining_and_grows_without_restart",
        )?;
    assert_eq!(budget.snapshot().active, 2);
    drop((third, admitted));
    assert_eq!(budget.snapshot().active, 0);

    Ok(())
}

#[tokio::test]
async fn processing_budget_preserves_fifo_and_cancellation_releases_waiters(
) -> Result<(), Box<dyn std::error::Error>> {
    let budget = Arc::new(ProcessingBudget::new(2));
    budget.configure(1, 8);
    let token = CancellationToken::new();
    let running = budget.acquire(WorkerKind::Thumbnails, &token).await.ok_or(
        "Expected a value in processing_budget_preserves_fifo_and_cancellation_releases_waiters",
    )?;
    let canceled = CancellationToken::new();
    let first = request(&budget, WorkerKind::Colors, &canceled);
    wait_for_queue(&budget, 1).await?;
    let second = request(&budget, WorkerKind::Duplicates, &token);
    wait_for_queue(&budget, 2).await?;
    let last = request(&budget, WorkerKind::Thumbnails, &token);
    wait_for_queue(&budget, 3).await?;
    canceled.cancel();
    assert!(first.await?.is_none());
    drop(running);
    let next = tokio::time::timeout(Duration::from_secs(2), second)
        .await
        ?
        ?
        .ok_or("Expected a value in processing_budget_preserves_fifo_and_cancellation_releases_waiters")?;
    assert!(!last.is_finished());
    drop(next);
    drop(tokio::time::timeout(Duration::from_secs(2), last).await??);
    assert_eq!(budget.snapshot().waiting, 0);
    assert_eq!(budget.snapshot().active, 0);

    Ok(())
}

#[tokio::test]
async fn processing_budget_skips_capped_workers_and_recovers_aborted_waiters(
) -> Result<(), Box<dyn std::error::Error>> {
    let budget = Arc::new(ProcessingBudget::new(8));
    budget.configure(2, 1);
    let token = CancellationToken::new();
    let thumbnail = budget.acquire(WorkerKind::Thumbnails, &token).await.ok_or(
        "Expected a value in processing_budget_skips_capped_workers_and_recovers_aborted_waiters",
    )?;
    let waiting_thumbnail = request(&budget, WorkerKind::Thumbnails, &token);
    wait_for_queue(&budget, 1).await?;
    let color = tokio::time::timeout(
        Duration::from_secs(2),
        budget.acquire(WorkerKind::Colors, &token),
    )
    .await?
    .ok_or(
        "Expected a value in processing_budget_skips_capped_workers_and_recovers_aborted_waiters",
    )?;
    waiting_thumbnail.abort();
    let _ = waiting_thumbnail.await;
    assert_eq!(budget.snapshot().waiting, 0);
    drop((thumbnail, color));
    let canceled = CancellationToken::new();
    canceled.cancel();
    assert!(budget
        .acquire(WorkerKind::Duplicates, &canceled)
        .await
        .is_none());
    assert_eq!(budget.snapshot().active, 0);

    Ok(())
}

#[tokio::test]
async fn processing_budget_serializes_duplicate_hashing_and_matching(
) -> Result<(), Box<dyn std::error::Error>> {
    let budget = Arc::new(ProcessingBudget::new(8));
    budget.configure(8, 8);
    let token = CancellationToken::new();
    let hashing = budget
        .acquire(WorkerKind::Duplicates, &token)
        .await
        .ok_or("Expected a value in processing_budget_serializes_duplicate_hashing_and_matching")?;
    let matching = request(&budget, WorkerKind::Duplicates, &token);
    wait_for_queue(&budget, 1).await?;
    assert!(!matching.is_finished());
    let color = budget
        .acquire(WorkerKind::Colors, &token)
        .await
        .ok_or("Expected a value in processing_budget_serializes_duplicate_hashing_and_matching")?;
    drop(hashing);
    drop(tokio::time::timeout(Duration::from_secs(2), matching).await??);
    drop(color);
    assert_eq!(budget.snapshot().active, 0);

    Ok(())
}

#[tokio::test]
async fn processing_budget_mixed_load_never_exceeds_shared_capacity(
) -> Result<(), Box<dyn std::error::Error>> {
    let budget = Arc::new(ProcessingBudget::new(8));
    budget.configure(3, 8);
    let peak = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut tasks = tokio::task::JoinSet::new();
    for index in 0..120 {
        let budget = budget.clone();
        let peak = peak.clone();
        tasks.spawn(async move {
            let worker = WorkerKind::ALL[index % 3];
            let _permit = budget
                .acquire(worker, &CancellationToken::new())
                .await
                .ok_or(
                "Expected a value in processing_budget_mixed_load_never_exceeds_shared_capacity",
            )?;
            let active = budget.snapshot().active;
            assert!(active <= 3);
            peak.fetch_max(active, std::sync::atomic::Ordering::SeqCst);
            tokio::task::yield_now().await;
            Ok::<(), &str>(())
        });
    }
    tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(result) = tasks.join_next().await {
            result??;
        }
        Ok::<(), Box<dyn std::error::Error>>(())
    })
    .await??;
    assert!(peak.load(std::sync::atomic::Ordering::SeqCst) > 1);
    assert_eq!(budget.snapshot().active, 0);
    assert_eq!(budget.snapshot().waiting, 0);

    Ok(())
}

#[test]
fn processing_budget_bounds_automatic_and_legacy_limits() {
    assert_eq!(ProcessingBudget::new(1).snapshot().limit, 1);
    assert_eq!(ProcessingBudget::new(64).snapshot().limit, 8);
    let budget = ProcessingBudget::new(1);
    budget.configure(usize::MAX, usize::MAX);
    assert_eq!(budget.snapshot().limit, 8);
    assert_eq!(budget.snapshot().thumbnail_limit, 8);
    budget.configure(4, 1);
    assert_eq!(budget.snapshot().thumbnail_limit, 1);
}
