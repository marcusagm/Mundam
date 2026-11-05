//! Low-frequency tool checks recover suspended dependencies independently of asset events.

use crate::feature::processing::tools::ProcessingToolService;
use std::{sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;

/// Checks immediately at startup and once per minute; an admitted probe batch drains normally.
pub fn start(
    service: Arc<ProcessingToolService>,
    token: CancellationToken,
) -> tauri::async_runtime::JoinHandle<()> {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! { _ = token.cancelled() => break, _ = interval.tick() => {} }
            if let Err(error) = service.check().await {
                tracing::warn!(%error, "Extraction capability check failed");
            }
        }
    })
}
