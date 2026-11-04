//! Shared matching execution for automatic consumption and explicit scans.

use super::coordinator::DuplicateProcessingCoordinator;
use crate::core::{duplicates::matching::*, repository::DuplicatesRepository, AppError, AppResult};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// The caller owns coordinator and pause admission, including while CPU work drains.
///
/// # Errors
/// Persists bounded comparison failures; obsolete or canceled work produces no publication.
pub async fn process(
    repository: Arc<dyn DuplicatesRepository>,
    coordinator: Arc<DuplicateProcessingCoordinator>,
    kind: MatchingKind,
    automatic_enabled: bool,
    token: &CancellationToken,
) -> AppResult<Option<usize>> {
    if token.is_cancelled() {
        return Ok(None);
    }
    let Some(_resources) = coordinator
        .budget
        .acquire(
            crate::core::processing::control::WorkerKind::Duplicates,
            token,
        )
        .await
    else {
        return Ok(None);
    };
    let claim_started = std::time::Instant::now();
    let Some(input) = repository.claim_matching(kind, automatic_enabled).await? else {
        return Ok(None);
    };
    let claim_ms = claim_started.elapsed().as_millis() as u64;
    let input = Arc::new(input);
    let computation = token.child_token();
    let _activity = coordinator.watch_matching(&input.ticket, computation.clone());
    let stop = CancellationToken::new();
    let _stop = stop.clone().drop_guard();
    let heartbeat_repository = repository.clone();
    let heartbeat_ticket = input.ticket.clone();
    let heartbeat_computation = computation.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = stop.cancelled() => break,
                _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {}
            }
            match heartbeat_repository.renew_matching(&heartbeat_ticket).await {
                Ok(true) => {}
                _ => {
                    heartbeat_computation.cancel();
                    break;
                }
            }
        }
        if let Err(error) = heartbeat_repository
            .release_matching(&heartbeat_ticket, None)
            .await
        {
            tracing::warn!(%error, "Matching reservation will recover through expiry");
        }
    });
    let result = async {
        let worker_input = input.clone();
        let worker_token = computation.clone();
        let (groups, compare_ms) = tokio::task::spawn_blocking(move || {
            let started = std::time::Instant::now();
            let groups = super::matching::compare(&worker_input, &worker_token);
            (groups, started.elapsed().as_millis() as u64)
        })
        .await
        .map_err(|error| AppError::Internal(format!("Matching computation failed: {error}")))?;
        let Some(groups) = groups else {
            return Ok(None);
        };
        if computation.is_cancelled() {
            return Ok(None);
        }
        let count = groups.len();
        let publish_started = std::time::Instant::now();
        let published = repository.publish_matching(&input, groups).await?;
        tracing::info!(
            phase = kind.as_str(),
            revision = input.ticket.revision,
            fingerprints = input.fingerprints.len(),
            groups = count,
            claim_ms,
            compare_ms,
            publish_ms = publish_started.elapsed().as_millis() as u64,
            published,
            "Duplicate matching measurement"
        );
        if published {
            Ok(Some(count))
        } else {
            Ok(None)
        }
    }
    .await;
    repository
        .release_matching(
            &input.ticket,
            result.as_ref().err().map(ToString::to_string),
        )
        .await?;
    result
}

/// Explicit scans wait through another owner or a bounded retry without losing cancellation.
///
/// # Errors
/// Returns terminal comparison failures and storage errors; canceled scans return zero.
pub async fn complete(
    repository: Arc<dyn DuplicatesRepository>,
    coordinator: Arc<DuplicateProcessingCoordinator>,
    kind: MatchingKind,
    token: &CancellationToken,
) -> AppResult<usize> {
    loop {
        if token.is_cancelled() {
            return Ok(0);
        }
        match process(repository.clone(), coordinator.clone(), kind, false, token).await {
            Ok(Some(count)) => return Ok(count),
            Ok(None) => {}
            Err(error) => {
                tracing::warn!(%error, "Manual comparison follows its persistent retry policy")
            }
        }
        let status = repository.matching_status().await?;
        if status.completed.iter().any(|phase| phase == kind.as_str()) {
            return Ok(0);
        }
        if status
            .failed_phases
            .iter()
            .any(|phase| phase == kind.as_str())
        {
            return Err(AppError::Internal(
                "Duplicate comparison reached its retry limit".into(),
            ));
        }
        tokio::select! {
            _ = token.cancelled() => return Ok(0),
            _ = tokio::time::sleep(std::time::Duration::from_millis(500)) => {}
        }
    }
}
