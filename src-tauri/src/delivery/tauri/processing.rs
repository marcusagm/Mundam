//! Passive desktop adapter for global worker activity and session pause intent.

use crate::core::{processing::control::WorkerKind, AppResult};
use crate::feature::processing::control::{ProcessingControlService, ProcessingSnapshot};
use std::sync::Arc;
use tauri::State;

/// Reads activity independently of viewport subscriptions and result events.
///
/// # Errors
/// Propagates persistence and settings read errors from the application service.
#[tauri::command]
pub async fn get_processing_snapshot(
    service: State<'_, Arc<ProcessingControlService>>,
) -> AppResult<ProcessingSnapshot> {
    service.snapshot().await
}

/// Changes future admissions while allowing existing operations to drain.
///
/// # Errors
/// Propagates failures to assemble the acknowledged snapshot.
#[tauri::command]
pub async fn set_processing_worker_paused(
    worker: WorkerKind,
    paused: bool,
    service: State<'_, Arc<ProcessingControlService>>,
) -> AppResult<ProcessingSnapshot> {
    service.set_paused(worker, paused).await
}

/// Queues terminal failures or unsupported assets without exposing private extractor logs.
/// # Errors
/// Propagates validation and storage failures from the processing service.
#[tauri::command]
pub async fn retry_processing_worker(
    worker: WorkerKind,
    scope: crate::core::processing::diagnostics::ProcessingRetryScope,
    service: State<'_, Arc<ProcessingControlService>>,
) -> AppResult<crate::feature::processing::control::ProcessingRetryAcknowledgement> {
    service.retry(worker, scope).await
}

/// Checks known extraction dependencies without changing worker pause intent.
/// # Errors
/// Propagates bounded probe and snapshot failures.
#[tauri::command]
pub async fn check_processing_tools(
    service: State<'_, Arc<ProcessingControlService>>,
) -> AppResult<ProcessingSnapshot> {
    service.check_tools().await
}
