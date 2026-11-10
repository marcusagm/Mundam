//! Retained, single-owner application shutdown state.

use crate::core::error::{AppError, AppResult};
use serde::Serialize;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokio::sync::Notify;

const PRESENTATION_DEADLINE: Duration = Duration::from_millis(1500);
const MINIMUM_PRESENTATION: Duration = Duration::from_millis(600);

/// Observable phase of the application shutdown.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ShutdownPhase {
    Idle,
    Requested,
    Draining,
    Finished,
}

/// Retained state exposed to the shutdown presentation.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShutdownSnapshot {
    pub revision: u64,
    pub phase: ShutdownPhase,
    pub degraded: bool,
}

struct ShutdownState {
    snapshot: ShutdownSnapshot,
    requested_at: Option<Instant>,
    presented_at: Option<Instant>,
    exit_authorized: bool,
}

/// Coordinates a single shutdown and its bounded presentation handshake.
pub struct ShutdownService {
    state: Mutex<ShutdownState>,
    changed: Notify,
}

impl Default for ShutdownService {
    fn default() -> Self {
        Self {
            state: Mutex::new(ShutdownState {
                snapshot: ShutdownSnapshot {
                    revision: 0,
                    phase: ShutdownPhase::Idle,
                    degraded: false,
                },
                requested_at: None,
                presented_at: None,
                exit_authorized: false,
            }),
            changed: Notify::new(),
        }
    }
}

impl ShutdownService {
    fn lock(&self) -> AppResult<std::sync::MutexGuard<'_, ShutdownState>> {
        self.state
            .lock()
            .map_err(|_| AppError::Internal("Shutdown state unavailable".into()))
    }

    /// Returns the retained presentation state.
    ///
    /// # Errors
    /// Returns an error when the state lock is poisoned.
    pub fn snapshot(&self) -> AppResult<ShutdownSnapshot> {
        Ok(self.lock()?.snapshot.clone())
    }

    /// Claims the one shutdown execution; repeated requests return false.
    ///
    /// # Errors
    /// Returns an error when the state lock is poisoned.
    pub fn request(&self) -> AppResult<bool> {
        let mut state = self.lock()?;
        if state.snapshot.phase != ShutdownPhase::Idle {
            return Ok(false);
        }
        state.requested_at = Some(Instant::now());
        state.snapshot.phase = ShutdownPhase::Requested;
        state.snapshot.revision += 1;
        Ok(true)
    }

    /// Runs a startup presentation action only while shutdown has not started.
    ///
    /// # Errors
    /// Returns an error if shutdown has started, the lock fails, or the action fails.
    pub fn while_running<T>(&self, action: impl FnOnce() -> AppResult<T>) -> AppResult<T> {
        let state = self.lock()?;
        if state.snapshot.phase != ShutdownPhase::Idle {
            return Err(AppError::ValidationFailed("Application is closing".into()));
        }
        action()
    }

    /// Advances to draining.
    ///
    /// # Errors
    /// Returns an error when the state lock is poisoned.
    pub fn draining(&self) -> AppResult<ShutdownSnapshot> {
        let mut state = self.lock()?;
        if state.snapshot.phase == ShutdownPhase::Requested {
            state.snapshot.phase = ShutdownPhase::Draining;
            state.snapshot.revision += 1;
        }
        Ok(state.snapshot.clone())
    }

    /// Records a timely presentation acknowledgement once.
    ///
    /// # Errors
    /// Rejects acknowledgements outside the bounded presentation interval.
    pub fn presented(&self) -> AppResult<()> {
        let mut state = self.lock()?;
        let now = Instant::now();
        let requested = state
            .requested_at
            .ok_or_else(|| AppError::ValidationFailed("Shutdown has not been requested".into()))?;
        if state.presented_at.is_some() {
            return Ok(());
        }
        if now.duration_since(requested) > PRESENTATION_DEADLINE || state.exit_authorized {
            return Err(AppError::ValidationFailed(
                "Shutdown presentation deadline elapsed".into(),
            ));
        }
        state.presented_at = Some(now);
        drop(state);
        self.changed.notify_one();
        Ok(())
    }

    /// Marks draining complete and retains whether any registered task failed.
    ///
    /// # Errors
    /// Returns an error when the state lock is poisoned.
    pub fn finish(&self, degraded: bool) -> AppResult<ShutdownSnapshot> {
        let mut state = self.lock()?;
        if state.snapshot.phase == ShutdownPhase::Draining {
            state.snapshot.phase = ShutdownPhase::Finished;
            state.snapshot.degraded = degraded;
            state.snapshot.revision += 1;
        }
        Ok(state.snapshot.clone())
    }

    /// Waits only for the bounded presentation handshake and its minimum display time.
    ///
    /// # Errors
    /// Returns an error when the state lock is poisoned or shutdown was not requested.
    pub async fn wait_for_presentation(&self) -> AppResult<()> {
        loop {
            let notified = self.changed.notified();
            let deadline = {
                let state = self.lock()?;
                let requested = state.requested_at.ok_or_else(|| {
                    AppError::ValidationFailed("Shutdown has not been requested".into())
                })?;
                match state.presented_at {
                    Some(presented) => presented + MINIMUM_PRESENTATION,
                    None => requested + PRESENTATION_DEADLINE,
                }
            };
            if Instant::now() >= deadline {
                return Ok(());
            }
            tokio::select! {
                _ = notified => {},
                _ = tokio::time::sleep_until(deadline.into()) => {},
            }
        }
    }

    /// Authorizes the coordinator's final native exit request.
    ///
    /// # Errors
    /// Returns an error when the state lock is poisoned.
    pub fn authorize_exit(&self) -> AppResult<()> {
        self.lock()?.exit_authorized = true;
        Ok(())
    }

    /// Returns whether final exit has been authorized by the coordinator.
    ///
    /// # Errors
    /// Returns an error when the state lock is poisoned.
    pub fn exit_authorized(&self) -> AppResult<bool> {
        Ok(self.lock()?.exit_authorized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_one_request_owns_shutdown() -> AppResult<()> {
        let service = ShutdownService::default();
        assert!(service.request()?);
        assert!(!service.request()?);
        assert!(service.while_running(|| Ok(())).is_err());
        let draining = service.draining()?;
        let finished = service.finish(true)?;
        assert!(finished.revision > draining.revision);
        assert!(finished.degraded);
        assert_eq!(service.draining()?.phase, ShutdownPhase::Finished);
        Ok(())
    }

    #[tokio::test]
    async fn presentation_wait_is_bounded_and_idempotent() -> AppResult<()> {
        let service = ShutdownService::default();
        service.request()?;
        service.presented()?;
        service.presented()?;
        let started = Instant::now();
        service.wait_for_presentation().await?;
        assert!(started.elapsed() >= Duration::from_millis(550));
        service.authorize_exit()?;
        assert!(service.exit_authorized()?);
        Ok(())
    }

    #[tokio::test]
    async fn missing_acknowledgement_does_not_block_indefinitely() -> AppResult<()> {
        let service = ShutdownService::default();
        service.request()?;
        let started = Instant::now();
        service.wait_for_presentation().await?;
        assert!(started.elapsed() >= Duration::from_millis(1450));
        assert!(started.elapsed() < Duration::from_secs(3));
        assert!(service.presented().is_err());
        Ok(())
    }

    #[test]
    fn concurrent_requests_have_one_owner() -> AppResult<()> {
        let service = std::sync::Arc::new(ShutdownService::default());
        let mut threads = Vec::new();
        for _ in 0..16 {
            let service = service.clone();
            threads.push(std::thread::spawn(move || service.request()));
        }
        let mut owners = 0;
        for thread in threads {
            let requested = thread
                .join()
                .map_err(|_| AppError::Internal("Request thread panicked".into()))??;
            owners += usize::from(requested);
        }
        assert_eq!(owners, 1);
        Ok(())
    }
}
