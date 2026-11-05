//! Explicit and periodic capability checks share one serialized, bounded probe batch.

use crate::core::{
    processing::{tools::*, ProcessingRepository},
    AppResult,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

/// Probes do not acquire worker permits, alter session pause or reset extraction failures.
pub struct ProcessingToolService {
    repository: Arc<dyn ProcessingRepository>,
    probe: Arc<dyn ExtractionToolProbe>,
    last_check: Mutex<Option<Instant>>,
}

impl ProcessingToolService {
    /// Shares observations with the queue adapter used by all workers.
    pub fn new(
        repository: Arc<dyn ProcessingRepository>,
        probe: Arc<dyn ExtractionToolProbe>,
    ) -> Self {
        Self {
            repository,
            probe,
            last_check: Mutex::new(None),
        }
    }

    /// Coalesces simultaneous requests and fences results captured before newer failures.
    /// # Errors
    /// Propagates observation read/write and probe infrastructure failures.
    pub async fn check(&self) -> AppResult<()> {
        let mut last = self.last_check.lock().await;
        if last.is_some_and(|checked| checked.elapsed() < Duration::from_secs(1)) {
            return Ok(());
        }
        let statuses = self.repository.tool_statuses().await?;
        let mut checks = Vec::new();
        for status in statuses {
            checks.push(ExtractionToolCheck {
                tool: status.tool,
                expected_revision: status.revision,
                available: self.probe.available(status.tool).await?,
            });
        }
        self.repository.record_tool_checks(checks).await?;
        *last = Some(Instant::now());
        Ok(())
    }
}
