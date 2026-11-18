//! Admission and lifetime accounting for application IPC, independent of its JSON arguments.

use crate::core::error::{AppError, AppResult};
use crate::lifecycle::LifecycleRegistry;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};
use tauri::{
    ipc::{CommandArg, CommandItem, InvokeError},
    Runtime,
};
use tokio::sync::watch;

#[derive(Default)]
struct AdmissionState {
    closed: bool,
    commands: BTreeMap<&'static str, usize>,
}

/// Retains every admitted command until its Rust invocation finishes.
pub struct IpcAdmission {
    state: Mutex<AdmissionState>,
    active: watch::Sender<usize>,
}

impl Default for IpcAdmission {
    fn default() -> Self {
        let (active, _) = watch::channel(0);
        Self {
            state: Mutex::new(AdmissionState::default()),
            active,
        }
    }
}

impl IpcAdmission {
    /// Atomically admits one invocation, or rejects it with a structured cancellation.
    /// # Errors
    /// Returns cancellation after shutdown has closed admission.
    pub fn enter(self: &Arc<Self>, command: &'static str) -> AppResult<IpcRequest> {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        if state.closed {
            tracing::debug!(command, "IPC command rejected after admission closed");
            return Err(AppError::Cancelled);
        }
        *state.commands.entry(command).or_default() += 1;
        self.active.send_replace(state.commands.values().sum());
        Ok(IpcRequest {
            admission: self.clone(),
            command,
        })
    }

    /// Closes admission before producer cancellation and returns the outstanding count.
    pub fn close(&self) -> usize {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        state.closed = true;
        state.commands.values().sum()
    }

    /// Names and counts of admitted commands, without request arguments or paths.
    pub fn active_commands(&self) -> BTreeMap<&'static str, usize> {
        self.state
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .commands
            .clone()
    }

    /// Whether admission is closed and no command can still access managed resources.
    pub fn is_drained(&self) -> bool {
        let state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        state.closed && state.commands.is_empty()
    }

    /// Closes admission and waits for all existing command scopes, without aborting them.
    pub async fn drain(&self) {
        self.close();
        let mut active = self.active.subscribe();
        loop {
            if *active.borrow_and_update() == 0 {
                return;
            }
            if active.changed().await.is_err() {
                unreachable!("IPC admission retains its watch sender");
            }
        }
    }

    /// Admits the drainer before boot; lifecycle keeps the bridge alive until it finishes.
    /// A rejected owner also closes admission, so an unmanaged gate cannot accept work.
    pub fn register_lifecycle_owner(self: &Arc<Self>, lifecycle: &Arc<LifecycleRegistry>) -> bool {
        let owner = self.clone();
        let cancellation = lifecycle.root_token();
        let admitted = lifecycle.spawn_unique("ipc_commands".into(), async move {
            cancellation.cancelled().await;
            let active = owner.close();
            tracing::info!(active, commands = ?owner.active_commands(), "IPC commands draining");
            owner.drain().await;
            tracing::info!("IPC commands drained");
        });
        if !admitted {
            self.close();
        }
        admitted
    }
}

/// Framework-extracted guard: frontend payloads never supply or bypass it.
/// Keep this named parameter in command signatures for their entire scope.
pub struct IpcRequest {
    admission: Arc<IpcAdmission>,
    command: &'static str,
}

impl std::fmt::Debug for IpcRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("IpcRequest")
            .field("command", &self.command)
            .finish_non_exhaustive()
    }
}

impl<'de, R: Runtime> CommandArg<'de, R> for IpcRequest {
    fn from_command(command: CommandItem<'de, R>) -> Result<Self, InvokeError> {
        let admission = command
            .message
            .state_ref()
            .try_get::<Arc<IpcAdmission>>()
            .ok_or_else(|| {
                InvokeError::from(AppError::Internal("IPC admission is unavailable".into()))
            })?;
        admission.enter(command.name).map_err(InvokeError::from)
    }
}

impl Drop for IpcRequest {
    fn drop(&mut self) {
        let mut state = self
            .admission
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(count) = state.commands.get_mut(self.command) {
            *count -= 1;
            if *count == 0 {
                state.commands.remove(self.command);
            }
        }
        self.admission
            .active
            .send_replace(state.commands.values().sum());
    }
}

#[cfg(test)]
#[path = "admission_tests.rs"]
mod tests;
