//! Admission and draining for asset and thumbnail protocol requests.

use std::sync::{Arc, Mutex};
use tokio::sync::watch;

#[derive(Default)]
struct State {
    closed: bool,
    active: usize,
}

/// Tracks admitted protocol handlers until they finish.
pub struct ProtocolAdmission {
    state: Mutex<State>,
    active: watch::Sender<usize>,
}

impl Default for ProtocolAdmission {
    fn default() -> Self {
        let (active, _) = watch::channel(0);
        Self {
            state: Mutex::new(State::default()),
            active,
        }
    }
}

impl ProtocolAdmission {
    /// Admits a request unless shutdown has closed admission.
    pub fn enter(self: &Arc<Self>) -> Option<ProtocolRequest> {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        if state.closed {
            return None;
        }
        state.active += 1;
        self.active.send_replace(state.active);
        Some(ProtocolRequest(self.clone()))
    }

    /// Atomically blocks new requests and returns the number already admitted.
    pub fn close(&self) -> usize {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        state.closed = true;
        state.active
    }

    #[cfg(test)]
    pub(super) fn active_count(&self) -> usize {
        self.state
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .active
    }

    /// Waits until every admitted handler has released its guard.
    pub async fn drain(&self) {
        let mut active = self.active.subscribe();
        loop {
            if *active.borrow_and_update() == 0 {
                return;
            }
            if active.changed().await.is_err() {
                unreachable!("protocol admission retains the sender");
            }
        }
    }
}

/// Releases an admitted request, including when its future is cancelled.
pub struct ProtocolRequest(Arc<ProtocolAdmission>);

impl Drop for ProtocolRequest {
    fn drop(&mut self) {
        let mut state = self
            .0
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        state.active -= 1;
        self.0.active.send_replace(state.active);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn closing_rejects_late_requests_and_waits_for_existing_guards(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let admission = Arc::new(ProtocolAdmission::default());
        let first = admission.enter().ok_or("first request rejected")?;
        let second = admission.enter().ok_or("second request rejected")?;
        assert_eq!(admission.close(), 2);
        assert!(admission.enter().is_none());
        assert!(
            tokio::time::timeout(Duration::from_millis(20), admission.drain())
                .await
                .is_err()
        );
        drop(first);
        assert!(
            tokio::time::timeout(Duration::from_millis(20), admission.drain())
                .await
                .is_err()
        );
        drop(second);
        tokio::time::timeout(Duration::from_secs(2), admission.drain()).await?;
        Ok(())
    }

    #[tokio::test]
    async fn close_without_requests_drains_immediately() -> Result<(), Box<dyn std::error::Error>> {
        let admission = Arc::new(ProtocolAdmission::default());
        assert_eq!(admission.close(), 0);
        tokio::time::timeout(Duration::from_secs(2), admission.drain()).await?;
        assert!(admission.enter().is_none());
        Ok(())
    }
}
