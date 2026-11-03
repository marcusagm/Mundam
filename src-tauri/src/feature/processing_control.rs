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
    sequence: u64,
    sampled: Option<Instant>,
    counts: Vec<ProcessingQueueCounts>,
    active_assets: Vec<Vec<String>>,
}

/// Serializes responses and shares short-lived queue samples between consumers.
pub struct ProcessingControlService {
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
        control: Arc<ProcessingControl>,
        repository: Arc<dyn ProcessingRepository>,
        settings: Arc<dyn SettingsRepository>,
        manual_scan: Arc<dyn ManualScanActivity>,
    ) -> Self {
        Self {
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
            for (index, kind) in [
                ProcessingKind::Thumbnail,
                ProcessingKind::Colors,
                ProcessingKind::Fingerprint,
            ]
            .into_iter()
            .enumerate()
            {
                counts.push(
                    self.repository
                        .queue_counts(kind, &active_assets[index])
                        .await?,
                );
            }
            cache.counts = counts;
            cache.active_assets = active_assets;
            cache.sampled = Some(Instant::now());
        }
        let automatic_duplicates = self.settings.load().await?.auto_detect_duplicates;
        let activity = self.control.snapshot();
        cache.sequence += 1;
        Ok(ProcessingSnapshot {
            instance_id: self.instance_id.clone(),
            sequence: cache.sequence,
            workers: activity
                .into_iter()
                .zip(cache.counts.iter().cloned())
                .map(|(activity, queue)| {
                    let automatic_enabled =
                        activity.worker != WorkerKind::Duplicates || automatic_duplicates;
                    WorkerSnapshot {
                        activity,
                        queue,
                        automatic_enabled,
                    }
                })
                .collect(),
            manual_scan: self.manual_scan.snapshot(),
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
}
