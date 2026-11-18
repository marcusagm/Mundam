//! Owns blocking native watcher operations until their completion is observed.

use crate::core::error::{AppError, AppResult};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

pub(super) enum NativeChange {
    Watch(PathBuf),
    Unwatch(Vec<PathBuf>),
    Stop,
}

pub(super) struct NativeWatcher {
    watcher: RecommendedWatcher,
    callback_released: oneshot::Receiver<()>,
}

// Notify's Windows and Linux Drop implementations request shutdown without joining
// their thread. Acknowledging destruction of the callback also covers its clones
// held by pending native requests, instead of treating Drop as completion.
struct CallbackLifetime {
    released: Option<oneshot::Sender<()>>,
    cancellation: CancellationToken,
}

impl CallbackLifetime {
    fn accepts_events(&self) -> bool {
        !self.cancellation.is_cancelled()
    }
}

impl Drop for CallbackLifetime {
    fn drop(&mut self) {
        if let Some(released) = self.released.take() {
            let _ = released.send(());
        }
    }
}

pub(super) struct NativeOutcome {
    pub watcher: Option<NativeWatcher>,
    pub added: Vec<PathBuf>,
    pub removed: Vec<PathBuf>,
    pub result: AppResult<()>,
    pub supervision_failed: bool,
}

pub(super) fn apply(
    watcher: Option<NativeWatcher>,
    change: NativeChange,
    sender: mpsc::Sender<Event>,
    cancellation: CancellationToken,
) -> NativeOutcome {
    let mut outcome = NativeOutcome {
        watcher,
        added: Vec::new(),
        removed: Vec::new(),
        result: Ok(()),
        supervision_failed: false,
    };
    outcome.result = update(&mut outcome, change, sender, cancellation);
    outcome
}

fn update(
    outcome: &mut NativeOutcome,
    change: NativeChange,
    sender: mpsc::Sender<Event>,
    cancellation: CancellationToken,
) -> AppResult<()> {
    if matches!(change, NativeChange::Stop) {
        // The receiver closes on cancellation, releasing callbacks blocked by a full queue.
        if let Some(watcher) = outcome.watcher.take() {
            tracing::debug!("Watcher native stop requested");
            drop(watcher.watcher);
            tracing::debug!("Watcher native drop returned; awaiting callback release");
            if let Err(error) = watcher.callback_released.blocking_recv() {
                outcome.supervision_failed = true;
                return Err(AppError::Internal(format!(
                    "Native watcher callback release failed: {error}"
                )));
            }
        }
        return Ok(());
    }
    if cancellation.is_cancelled() {
        return Err(AppError::Cancelled);
    }
    match change {
        NativeChange::Watch(path) => {
            if outcome.watcher.is_none() {
                let (released, callback_released) = oneshot::channel();
                let callback_lifetime = CallbackLifetime {
                    released: Some(released),
                    cancellation: cancellation.clone(),
                };
                let watcher = RecommendedWatcher::new(
                    move |result: notify::Result<Event>| {
                        if callback_lifetime.accepts_events() {
                            if let Ok(event) = result {
                                let _ = sender.blocking_send(event);
                            }
                        }
                    },
                    Config::default().with_poll_interval(std::time::Duration::from_millis(500)),
                )?;
                outcome.watcher = Some(NativeWatcher {
                    watcher,
                    callback_released,
                });
            }
            if cancellation.is_cancelled() {
                return Err(AppError::Cancelled);
            }
            if let Some(watcher) = outcome.watcher.as_mut() {
                watcher.watcher.watch(&path, RecursiveMode::Recursive)?;
                outcome.added.push(path);
            }
            Ok(())
        }
        NativeChange::Unwatch(paths) => {
            let mut failures = Vec::new();
            if let Some(watcher) = outcome.watcher.as_mut() {
                for path in paths {
                    match watcher.watcher.unwatch(&path) {
                        Ok(()) => outcome.removed.push(path),
                        Err(error) if matches!(error.kind, notify::ErrorKind::WatchNotFound) => {
                            outcome.removed.push(path);
                        }
                        Err(error) => failures.push(format!("{}: {error}", path.display())),
                    }
                }
            }
            if failures.is_empty() {
                Ok(())
            } else {
                Err(AppError::Internal(failures.join("; ")))
            }
        }
        NativeChange::Stop => Ok(()),
    }
}
