//! Session-scoped admission and activity, independent of persistence and desktop delivery.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// User-visible executor groups; duplicate hashing and matching share one control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerKind {
    Thumbnails,
    Colors,
    Duplicates,
}

/// Read-only progress from the existing manual scan, without command-service dependencies.
#[derive(Clone, Serialize)]
pub struct ManualScanProgress {
    /// Includes a requested scan waiting for common duplicate admission.
    pub is_running: bool,
    /// Existing scan phase, including preparation and cancellation.
    pub phase: String,
    /// Files analyzed during manual fingerprint generation.
    pub processed: usize,
    /// Known file total for that phase.
    pub total: usize,
}

/// Lets global activity queries consume progress without invoking command handlers.
pub trait ManualScanActivity: Send + Sync {
    /// Reads runtime facts without starting or mutating a scan.
    fn snapshot(&self) -> ManualScanProgress;
}

impl WorkerKind {
    /// Stable ordering for snapshots and the activity panel.
    pub const ALL: [Self; 3] = [Self::Thumbnails, Self::Colors, Self::Duplicates];
}

/// Queue outcomes exclude inactive assets and assets currently admitted by an executor.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ProcessingQueueCounts {
    /// Eligible for admission now.
    pub pending: u64,
    /// Deferred by the bounded retry policy.
    pub retry_wait: u64,
    /// Exhausted attempts for the current source generation.
    pub failed: u64,
    /// Unsupported work for the current source generation.
    pub skipped: u64,
}

/// Runtime facts never infer completion from an empty queue or asset lifecycle state.
#[derive(Debug, Clone, Serialize)]
pub struct WorkerActivity {
    /// Executor controlled by this row.
    pub worker: WorkerKind,
    /// False before startup and after the orchestration task exits.
    pub online: bool,
    /// Pausing drains admitted operations instead of canceling native work.
    pub paused: bool,
    /// Includes admitted work awaiting its first persistence check.
    pub active: usize,
    /// Algorithm phase, without source paths or raw provider errors.
    pub phase: Option<String>,
    /// A follow-up operation is scheduled even when no asset is currently running.
    pub scheduled: bool,
    /// Internal identities used to avoid counting running assets as queued.
    #[serde(skip)]
    pub active_assets: Vec<String>,
}

#[derive(Default)]
struct WorkerState {
    online: bool,
    paused: bool,
    scheduled: bool,
    next_operation: u64,
    operations: BTreeMap<u64, (String, Option<String>)>,
}

/// A single lock linearizes pause acknowledgments and new execution admissions.
#[derive(Default)]
pub struct ProcessingControl {
    /// Shared capacity for thumbnail, palette, fingerprint and matching operations.
    pub budget: Arc<super::budget::ProcessingBudget>,
    states: Mutex<BTreeMap<WorkerKind, WorkerState>>,
}

impl ProcessingControl {
    /// Retains pending follow-up work separately from executing operations.
    pub fn set_scheduled(&self, worker: WorkerKind, scheduled: bool) {
        self.states
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .entry(worker)
            .or_default()
            .scheduled = scheduled;
    }
    /// Updates session intent; previously admitted work keeps its permit until completion.
    pub fn set_paused(&self, worker: WorkerKind, paused: bool) {
        self.states
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .entry(worker)
            .or_default()
            .paused = paused;
    }

    /// Allows orchestrators to sleep without repeatedly scanning a paused queue.
    pub fn is_paused(&self, worker: WorkerKind) -> bool {
        self.states
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .get(&worker)
            .is_some_and(|state| state.paused)
    }

    /// Acquires a drainable operation or rejects it atomically after a pause request.
    pub fn try_admit(
        self: &Arc<Self>,
        worker: WorkerKind,
        phase: &str,
        asset: Option<&str>,
    ) -> Option<ProcessingPermit> {
        let mut states = self
            .states
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let state = states.entry(worker).or_default();
        if state.paused {
            return None;
        }
        state.next_operation += 1;
        let operation = state.next_operation;
        state
            .operations
            .insert(operation, (phase.into(), asset.map(str::to_owned)));
        Some(ProcessingPermit {
            control: self.clone(),
            worker,
            operation,
        })
    }

    /// Marks a running orchestration task; unwinding and shutdown clear its online state.
    pub fn start(self: &Arc<Self>, worker: WorkerKind) -> WorkerLifetime {
        self.states
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .entry(worker)
            .or_default()
            .online = true;
        WorkerLifetime {
            control: self.clone(),
            worker,
        }
    }

    /// Captures all runtime rows under one lock.
    pub fn snapshot(&self) -> Vec<WorkerActivity> {
        let states = self
            .states
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        WorkerKind::ALL
            .into_iter()
            .map(|worker| {
                let state = states.get(&worker);
                WorkerActivity {
                    worker,
                    online: state.is_some_and(|state| state.online),
                    paused: state.is_some_and(|state| state.paused),
                    active: state.map_or(0, |state| state.operations.len()),
                    scheduled: state.is_some_and(|state| state.scheduled),
                    phase: state
                        .and_then(|state| state.operations.values().next())
                        .map(|(phase, _)| phase.clone()),
                    active_assets: state
                        .into_iter()
                        .flat_map(|state| state.operations.values())
                        .filter_map(|(_, asset)| asset.clone())
                        .collect(),
                }
            })
            .collect()
    }
}

/// RAII ownership keeps activity accurate across errors, cancellation and unwinding.
pub struct ProcessingPermit {
    control: Arc<ProcessingControl>,
    worker: WorkerKind,
    operation: u64,
}

impl ProcessingPermit {
    /// Updates a long operation's phase without releasing admission during a pause.
    pub fn set_phase(&self, phase: &str) {
        let mut states = self
            .control
            .states
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(operation) = states
            .entry(self.worker)
            .or_default()
            .operations
            .get_mut(&self.operation)
        {
            operation.0 = phase.into();
        }
    }
}

impl Drop for ProcessingPermit {
    fn drop(&mut self) {
        self.control
            .states
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .entry(self.worker)
            .or_default()
            .operations
            .remove(&self.operation);
    }
}

/// Lifecycle ownership distinguishes an idle worker from a stopped worker.
pub struct WorkerLifetime {
    control: Arc<ProcessingControl>,
    worker: WorkerKind,
}

impl Drop for WorkerLifetime {
    fn drop(&mut self) {
        let mut states = self
            .control
            .states
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let state = states.entry(self.worker).or_default();
        state.online = false;
        state.scheduled = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn processing_pause_blocks_new_admissions_and_drains_existing_operations() {
        let control = Arc::new(ProcessingControl::default());
        let lifetime = control.start(WorkerKind::Thumbnails);
        let activity = control.try_admit(WorkerKind::Thumbnails, "thumbnail", Some("asset"));
        control.set_paused(WorkerKind::Thumbnails, true);
        assert!(control
            .try_admit(WorkerKind::Thumbnails, "thumbnail", None)
            .is_none());
        assert_eq!(control.snapshot()[0].active, 1);
        assert!(control.snapshot()[0].paused);
        assert!(control.snapshot()[0].online);
        drop(activity);
        assert_eq!(control.snapshot()[0].active, 0);
        control.set_paused(WorkerKind::Thumbnails, false);
        assert!(control
            .try_admit(WorkerKind::Thumbnails, "thumbnail", None)
            .is_some());
        drop(lifetime);
        assert!(!control.snapshot()[0].online);
    }

    #[test]
    fn processing_activity_is_released_after_unwinding_and_controls_are_independent() {
        let control = Arc::new(ProcessingControl::default());
        control.set_paused(WorkerKind::Colors, true);
        let failed = std::panic::catch_unwind({
            let control = control.clone();
            move || {
                let activity = control.try_admit(WorkerKind::Duplicates, "manual_scan", None);
                assert!(activity.is_some());
                panic!("Simulated provider panic");
            }
        });
        assert!(failed.is_err());
        assert_eq!(control.snapshot()[2].active, 0);
        assert!(control
            .try_admit(WorkerKind::Colors, "colors", None)
            .is_none());
        assert!(control
            .try_admit(WorkerKind::Thumbnails, "thumbnail", None)
            .is_some());
    }
}
