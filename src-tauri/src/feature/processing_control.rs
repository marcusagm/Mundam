//! Global processing snapshots and session controls, without asset invalidation or per-job events.

use crate::core::{
    processing::{ProcessingKind, ProcessingRepository},
    processing_control::{
        ManualScanActivity, ManualScanProgress, ProcessingControl, ProcessingQueueCounts,
        WorkerActivity, WorkerKind,
    },
    settings::SettingsRepository,
    AppResult,
};
use serde::Serialize;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

/// One row combines actual runtime admissions with durable queue outcomes.
#[derive(Clone, Serialize)]
pub struct WorkerSnapshot {
    /// Safe grouped causes for the current catalog sources.
    pub diagnostics: Vec<crate::core::processing_diagnostics::ProcessingDiagnostic>,
    /// Runtime lifecycle, pause intent and activity.
    #[serde(flatten)]
    pub activity: WorkerActivity,
    /// Pending and terminal outcomes, excluding admitted assets.
    #[serde(flatten)]
    pub queue: ProcessingQueueCounts,
    /// Automatic duplicates can be disabled independently of session pause.
    pub automatic_enabled: bool,
}

/// Full replacement snapshot; instance and sequence fence stale frontend responses.
#[derive(Clone, Serialize)]
pub struct ProcessingSnapshot {
    /// Durable comparison work is distinct from per-asset fingerprint queues.
    pub matching: crate::core::matching::MatchingStatus,
    /// Changes when this application service restarts.
    pub instance_id: String,
    /// Increases for each emitted response within this instance.
    pub sequence: u64,
    /// Worker rows have stable identities and ordering.
    pub workers: Vec<WorkerSnapshot>,
    /// Existing manual scan progress remains visible without a second execution path.
    pub manual_scan: ManualScanProgress,
}

#[derive(Default)]
struct SnapshotCache {
    diagnostics: Vec<Vec<crate::core::processing_diagnostics::ProcessingDiagnostic>>,
    sequence: u64,
    sampled: Option<Instant>,
    counts: Vec<ProcessingQueueCounts>,
    active_assets: Vec<Vec<String>>,
}

/// Serializes responses and shares short-lived queue samples between consumers.
pub struct ProcessingControlService {
    duplicates: Arc<dyn crate::core::repository::DuplicatesRepository>,
    control: Arc<ProcessingControl>,
    repository: Arc<dyn ProcessingRepository>,
    settings: Arc<dyn SettingsRepository>,
    manual_scan: Arc<dyn ManualScanActivity>,
    instance_id: String,
    cache: Mutex<SnapshotCache>,
}

impl ProcessingControlService {
    /// Shares admission state with all workers and the manual duplicate service.
    pub fn new(
        duplicates: Arc<dyn crate::core::repository::DuplicatesRepository>,
        control: Arc<ProcessingControl>,
        repository: Arc<dyn ProcessingRepository>,
        settings: Arc<dyn SettingsRepository>,
        manual_scan: Arc<dyn ManualScanActivity>,
    ) -> Self {
        Self {
            duplicates,
            control,
            repository,
            settings,
            manual_scan,
            instance_id: uuid::Uuid::new_v4().to_string(),
            cache: Mutex::new(SnapshotCache::default()),
        }
    }

    /// Reconciles durable queues while retaining a strict response order across callers.
    ///
    /// # Errors
    /// Propagates queue and settings read failures instead of reporting a healthy idle state.
    pub async fn snapshot(&self) -> AppResult<ProcessingSnapshot> {
        let mut cache = self.cache.lock().await;
        let activity = self.control.snapshot();
        let active_assets: Vec<_> = activity
            .iter()
            .map(|row| row.active_assets.clone())
            .collect();
        if cache
            .sampled
            .is_none_or(|sample| sample.elapsed() >= Duration::from_secs(1))
            || cache.active_assets != active_assets
        {
            let mut counts = Vec::new();
            let mut diagnostics = Vec::new();
            for (index, kind) in [
                ProcessingKind::Thumbnail,
                ProcessingKind::Colors,
                ProcessingKind::Fingerprint,
            ]
            .into_iter()
            .enumerate()
            {
                diagnostics.push(self.repository.diagnostics(kind).await?);
                counts.push(
                    self.repository
                        .queue_counts(kind, &active_assets[index])
                        .await?,
                );
            }
            cache.counts = counts;
            cache.diagnostics = diagnostics;
            cache.active_assets = active_assets;
            cache.sampled = Some(Instant::now());
        }
        let automatic_duplicates = self.settings.load().await?.auto_detect_duplicates;
        let matching = self.duplicates.matching_status().await?;
        let mut activity = self.control.snapshot();
        if let Some(duplicates) = activity
            .iter_mut()
            .find(|activity| activity.worker == WorkerKind::Duplicates)
        {
            duplicates.scheduled =
                matching.pending > 0 || matching.retry_wait > 0 || matching.manual_pending;
        }
        let mut manual_scan = self.manual_scan.snapshot();
        if !manual_scan.is_running && matching.manual_pending {
            manual_scan.is_running = true;
            manual_scan.phase = if matching.manual_cancel_requested {
                "cancel_requested".into()
            } else if activity.iter().any(|activity| {
                activity.worker == WorkerKind::Duplicates && activity.paused && activity.active == 0
            }) {
                "paused".into()
            } else if matching.retry_wait > 0 {
                "retry_wait".into()
            } else {
                matching
                    .manual_phase
                    .clone()
                    .unwrap_or_else(|| "recovering".into())
            };
            manual_scan.processed = 0;
            manual_scan.total = 0;
        }
        if manual_scan.is_running
            && matching.manual_pending
            && matching.retry_wait > 0
            && matches!(
                manual_scan.phase.as_str(),
                "exact_matching" | "visual_matching"
            )
        {
            manual_scan.phase = "retry_wait".into();
        }
        cache.sequence += 1;
        Ok(ProcessingSnapshot {
            matching,
            instance_id: self.instance_id.clone(),
            sequence: cache.sequence,
            workers: activity
                .into_iter()
                .zip(cache.counts.iter().cloned())
                .enumerate()
                .map(|(index, (activity, queue))| {
                    let automatic_enabled =
                        activity.worker != WorkerKind::Duplicates || automatic_duplicates;
                    WorkerSnapshot {
                        diagnostics: cache.diagnostics[index].clone(),
                        activity,
                        queue,
                        automatic_enabled,
                    }
                })
                .collect(),
            manual_scan,
        })
    }

    /// Acknowledges pause intent atomically; admitted operations finish normally.
    ///
    /// # Errors
    /// Returns a snapshot read error if the new state cannot be reported.
    pub async fn set_paused(
        &self,
        worker: WorkerKind,
        paused: bool,
    ) -> AppResult<ProcessingSnapshot> {
        self.control.set_paused(worker, paused);
        self.snapshot().await
    }

    /// Resets only explicitly selected terminal outcomes, without changing pause or automation.
    /// # Errors
    /// Propagates atomic queue writes or snapshot read failures; neither implies a completed retry.
    pub async fn retry(
        &self,
        worker: WorkerKind,
        scope: crate::core::processing_diagnostics::ProcessingRetryScope,
    ) -> AppResult<ProcessingRetryAcknowledgement> {
        let kind = match worker {
            WorkerKind::Thumbnails => ProcessingKind::Thumbnail,
            WorkerKind::Colors => ProcessingKind::Colors,
            WorkerKind::Duplicates => ProcessingKind::Fingerprint,
        };
        let counts = self.repository.retry(kind, scope).await?;
        self.cache.lock().await.sampled = None;
        Ok(ProcessingRetryAcknowledgement {
            snapshot: self.snapshot().await?,
            counts,
        })
    }
}

/// Acknowledges how much work was actually queued, separately from eventual execution outcomes.
#[derive(Serialize)]
pub struct ProcessingRetryAcknowledgement {
    pub snapshot: ProcessingSnapshot,
    #[serde(flatten)]
    pub counts: crate::core::processing_diagnostics::ProcessingRetryCounts,
}
