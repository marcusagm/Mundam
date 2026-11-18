//! Runs the debouncer and publisher as joined futures owned by one retained task.

use super::{debouncer::EventDebouncer, sensor::is_event_monitored};
use crate::core::events::AppEventBus;
use notify::Event;
use std::{collections::HashSet, path::PathBuf, sync::Arc};
use tokio::sync::{mpsc, Mutex};
use tokio_util::sync::CancellationToken;

#[derive(Default)]
pub(super) struct PipelineReport {
    pub raw_discarded: usize,
    pub pending_discarded: usize,
    pub domain_discarded: usize,
    pub publication_failures: usize,
}

pub(super) async fn run(
    mut input: mpsc::Receiver<Event>,
    event_bus: Arc<dyn AppEventBus>,
    watched_paths: Arc<Mutex<HashSet<PathBuf>>>,
    cancellation: CancellationToken,
) -> PipelineReport {
    let (sender, mut output) = mpsc::channel(100);
    let debounce = async {
        let mut debouncer = EventDebouncer::new(sender);
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(200));
        loop {
            tokio::select! {
                biased;
                _ = cancellation.cancelled() => {
                    input.close();
                    let mut discarded = 0;
                    while input.recv().await.is_some() {
                        discarded += 1;
                    }
                    return (discarded, debouncer.pending_count());
                }
                _ = interval.tick() => {
                    // Await an already-started filesystem operation; dropping its future
                    // would not prove the blocking operation had stopped.
                    debouncer.tick(std::time::Duration::from_millis(600)).await;
                }
                event = input.recv() => match event {
                    Some(event) => debouncer.handle_event(event).await,
                    None => return (0, debouncer.pending_count()),
                },
            }
        }
    };
    let publish = async {
        let mut discarded = 0;
        let mut failures = 0;
        loop {
            tokio::select! {
                biased;
                _ = cancellation.cancelled() => {
                    output.close();
                    while output.recv().await.is_some() {
                        discarded += 1;
                    }
                    return (discarded, failures);
                }
                event = output.recv() => {
                    let Some(event) = event else { return (discarded, failures) };
                    let registrations = watched_paths.lock().await;
                    if cancellation.is_cancelled() {
                        discarded += 1;
                    } else if is_event_monitored(&event, &registrations) {
                        if let Err(error) = event_bus.publish(event) {
                            tracing::warn!(%error, "Watcher event publication failed");
                            failures += 1;
                        }
                    }
                }
            }
        }
    };
    let ((raw_discarded, pending_discarded), (domain_discarded, publication_failures)) =
        tokio::join!(debounce, publish);
    PipelineReport {
        raw_discarded,
        pending_discarded,
        domain_discarded,
        publication_failures,
    }
}
