//! Observable application startup, independent of the desktop window adapter.
//!
//! Retaining a snapshot prevents late subscribers from missing boot milestones.

use crate::core::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

/// Ordered milestones shared with the startup presentation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StartupStage {
    Preparing,
    Database,
    Media,
    Library,
    BackendReady,
    Appearance,
    Content,
    Interface,
    Ready,
}

/// Serializable readiness contract; elapsed time is measured by each client.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupSnapshot {
    pub revision: u64,
    pub stage: StartupStage,
    pub backend_ready: bool,
    pub failed: bool,
    pub error_code: Option<String>,
}

/// Owns the monotonic boot state without depending on Tauri or the database.
pub struct StartupService(Mutex<StartupSnapshot>);

impl Default for StartupService {
    fn default() -> Self {
        Self(Mutex::new(StartupSnapshot {
            revision: 0,
            stage: StartupStage::Preparing,
            backend_ready: false,
            failed: false,
            error_code: None,
        }))
    }
}

impl StartupService {
    /// Reads retained state even when no event listener existed during boot.
    ///
    /// # Errors
    /// Returns an internal error if another thread poisoned the state lock.
    pub fn snapshot(&self) -> AppResult<StartupSnapshot> {
        self.0
            .lock()
            .map(|state| state.clone())
            .map_err(|_| AppError::Internal("Startup state unavailable".into()))
    }

    /// Advances milestones monotonically and preserves terminal failures.
    ///
    /// # Errors
    /// Returns an internal error if the state lock is poisoned.
    pub fn advance(&self, stage: StartupStage) -> AppResult<StartupSnapshot> {
        let mut state = self
            .0
            .lock()
            .map_err(|_| AppError::Internal("Startup state unavailable".into()))?;
        if !state.failed && stage > state.stage {
            state.stage = stage;
            state.backend_ready = stage >= StartupStage::BackendReady;
            state.revision += 1;
        }
        Ok(state.clone())
    }

    /// Rejects presentation acknowledgements until backend dependencies exist.
    ///
    /// # Errors
    /// Returns validation errors for premature or invalid frontend milestones.
    pub fn report_frontend(&self, stage: StartupStage) -> AppResult<StartupSnapshot> {
        let state = self.snapshot()?;
        if !state.backend_ready
            || state.failed
            || stage < StartupStage::Appearance
            || stage >= StartupStage::Ready
        {
            return Err(AppError::ValidationFailed(
                "Startup is not ready for this milestone".into(),
            ));
        }
        self.advance(stage)
    }

    /// Validates presentation readiness before the desktop adapter reveals a window.
    ///
    /// # Errors
    /// Rejects incomplete or failed frontend initialization.
    pub fn ensure_presentable(&self) -> AppResult<()> {
        let state = self.snapshot()?;
        if state.failed || state.stage < StartupStage::Interface {
            return Err(AppError::ValidationFailed(
                "The interface is not ready".into(),
            ));
        }
        Ok(())
    }

    /// Retains a safe failure code while diagnostic details remain in tracing.
    ///
    /// # Errors
    /// Returns an internal error if the state lock is poisoned.
    pub fn fail(&self, code: &str) -> AppResult<StartupSnapshot> {
        let mut state = self
            .0
            .lock()
            .map_err(|_| AppError::Internal("Startup state unavailable".into()))?;
        if !state.failed {
            state.failed = true;
            state.error_code = Some(code.into());
            state.revision += 1;
        }
        Ok(state.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn late_readers_observe_backend_readiness_and_monotonic_stages() -> AppResult<()> {
        let service = StartupService::default();
        service.advance(StartupStage::BackendReady)?;
        let content = service.report_frontend(StartupStage::Content)?;
        let stale = service.report_frontend(StartupStage::Appearance)?;
        assert!(stale.backend_ready);
        assert_eq!(content.revision, stale.revision);
        assert_eq!(stale.stage, StartupStage::Content);
        Ok(())
    }

    #[test]
    fn frontend_cannot_complete_before_backend_or_after_failure() -> AppResult<()> {
        let service = StartupService::default();
        assert!(service.report_frontend(StartupStage::Ready).is_err());
        service.fail("DATABASE_STARTUP_FAILED")?;
        let state = service.advance(StartupStage::BackendReady)?;
        assert!(state.failed);
        assert!(!state.backend_ready);
        assert!(service.report_frontend(StartupStage::Ready).is_err());
        Ok(())
    }

    #[test]
    fn backend_readiness_alone_does_not_allow_window_presentation() -> AppResult<()> {
        let service = StartupService::default();
        service.advance(StartupStage::BackendReady)?;
        assert!(service.ensure_presentable().is_err());
        service.report_frontend(StartupStage::Interface)?;
        assert!(service.ensure_presentable().is_ok());
        service.fail("FRONTEND_STARTUP_FAILED")?;
        assert!(service.ensure_presentable().is_err());
        Ok(())
    }
}
