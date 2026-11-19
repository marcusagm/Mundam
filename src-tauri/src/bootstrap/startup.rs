//! Desktop boot composition and event adaptation for retained startup state.

use crate::core::error::{AppError, AppResult};
use crate::feature::startup::{StartupService, StartupSnapshot, StartupStage};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

/// Retains the boot supervisor so shutdown can await late service registration.
pub struct StartupTask(tokio::sync::Mutex<StartupCompletion>);

enum StartupCompletion {
    Running(tauri::async_runtime::JoinHandle<()>),
    Finished(bool),
}

impl StartupTask {
    fn new(handle: tauri::async_runtime::JoinHandle<()>) -> Self {
        Self(tokio::sync::Mutex::new(StartupCompletion::Running(handle)))
    }

    /// Bounds shutdown's boot wait without aborting admitted native work or losing its owner.
    /// A timeout prevents explicit pool closure even if the supervisor finishes later.
    pub async fn wait(&self) -> bool {
        self.wait_within(Duration::from_secs(5)).await
    }

    async fn wait_within(&self, budget: Duration) -> bool {
        let completion = async {
            let mut state = self.0.lock().await;
            if let StartupCompletion::Running(handle) = &mut *state {
                let clean = match handle.await {
                    Ok(()) => true,
                    Err(error) => {
                        tracing::error!(%error, "Startup supervisor failed during shutdown");
                        false
                    }
                };
                *state = StartupCompletion::Finished(clean);
            }
            match *state {
                StartupCompletion::Finished(clean) => clean,
                StartupCompletion::Running(_) => false,
            }
        };
        match tokio::time::timeout(budget, completion).await {
            Ok(clean) => clean,
            Err(_) => {
                tracing::warn!(budget_ms = budget.as_millis() as u64,
                    "Startup supervisor still active; retaining owner and skipping explicit pool close");
                false
            }
        }
    }
}

/// Each admitted phase finishes before cancellation is checked; no later phase is started.
async fn run_stages<F, Work>(
    token: &tokio_util::sync::CancellationToken,
    mut initialize: F,
) -> AppResult<()>
where
    F: FnMut(StartupStage) -> Work,
    Work: std::future::Future<Output = AppResult<()>>,
{
    for stage in [
        StartupStage::Preparing,
        StartupStage::Database,
        StartupStage::Media,
        StartupStage::Library,
        StartupStage::BackendReady,
    ] {
        if token.is_cancelled() {
            tracing::info!(?stage, "Startup stopped before admitting next phase");
            return Ok(());
        }
        initialize(stage).await?;
    }
    Ok(())
}

/// Registers startup state before configured windows can issue IPC requests.
pub fn register_state<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder.manage(StartupService::default())
}

/// Launches a supervised, non-blocking boot using the pre-registered state.
pub fn start(application: &AppHandle) {
    let handle = application.clone();
    let task = tauri::async_runtime::spawn(async move {
        let startup_handle = handle.clone();
        let initialization =
            tauri::async_runtime::spawn(async move { initialize(&startup_handle).await });
        let failure = match initialization.await {
            Ok(Ok(())) => return,
            Ok(Err(error)) => error.to_string(),
            Err(error) => error.to_string(),
        };
        if handle
            .state::<std::sync::Arc<crate::lifecycle::LifecycleRegistry>>()
            .root_token()
            .is_cancelled()
        {
            return;
        }
        tracing::error!(%failure, "Application startup failed");
        match handle
            .state::<StartupService>()
            .fail("BACKEND_STARTUP_FAILED")
        {
            Ok(snapshot) => publish(&handle, &snapshot),
            Err(error) => tracing::error!(%error, "Unable to retain startup failure"),
        }
    });
    application.manage(StartupTask::new(task));
}

/// Publishes a retained snapshot; a missed event is recoverable through querying.
pub fn publish(application: &AppHandle, snapshot: &StartupSnapshot) {
    if let Err(error) = application.emit("startup:changed", snapshot) {
        tracing::warn!(%error, "Unable to publish startup progress");
    }
}

/// Keeps auxiliary-window disposal separate from application shutdown.
/// A splash closed by the user during boot must still terminate the hidden main.
pub fn should_shutdown_on_close(application: &AppHandle, label: &str) -> bool {
    match label {
        "main" => true,
        "splashscreen" => application
            .try_state::<StartupService>()
            .and_then(|service| service.snapshot().ok())
            .map(|snapshot| snapshot.stage != StartupStage::Ready || snapshot.failed)
            .unwrap_or(true),
        _ => false,
    }
}

fn advance(application: &AppHandle, stage: StartupStage) -> AppResult<()> {
    application
        .state::<crate::feature::shutdown::ShutdownService>()
        .while_running(|| {
            tracing::info!(?stage, "Application startup milestone");
            let snapshot = application.state::<StartupService>().advance(stage)?;
            publish(application, &snapshot);
            Ok(())
        })
}

/// Initializes dependencies after setup returns so the splash stays responsive.
///
/// # Errors
/// Propagates filesystem, database and startup-state failures to the boot supervisor.
async fn initialize(application: &AppHandle) -> AppResult<()> {
    let token = application
        .state::<std::sync::Arc<crate::lifecycle::LifecycleRegistry>>()
        .root_token();
    run_stages(&token, |stage| initialize_stage(application, stage)).await
}

async fn initialize_stage(application: &AppHandle, stage: StartupStage) -> AppResult<()> {
    match stage {
        StartupStage::Preparing => {
            let directory_handle = application.clone();
            tauri::async_runtime::spawn_blocking(move || {
                super::system::init_directories(&directory_handle)
            })
            .await
            .map_err(|error| AppError::Internal(error.to_string()))??;
        }
        StartupStage::Database => {
            super::system::init_settings(application);
            super::system::init_events(application);
            super::system::init_formats(application);
            advance(application, stage)?;
            super::database::init(application).await?;
        }
        StartupStage::Media => {
            advance(application, stage)?;
            super::streaming::init(application).await;
        }
        StartupStage::Library => {
            advance(application, stage)?;
            super::workers::init(application);
            super::library::init(application).await;
        }
        StartupStage::BackendReady => advance(application, stage)?,
        _ => {
            return Err(AppError::Internal(
                "Unexpected backend startup phase".into(),
            ))
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "startup_shutdown_tests.rs"]
mod shutdown_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_state_is_available_before_setup() -> Result<(), Box<dyn std::error::Error>> {
        let app = register_state(tauri::test::mock_builder())
            .invoke_handler(tauri::generate_handler![
                crate::delivery::tauri::startup::get_startup_state
            ])
            .build(crate::application_context::<tauri::test::MockRuntime>())?;
        let window = tauri::WebviewWindowBuilder::new(&app, "main", Default::default()).build()?;
        let response = tauri::test::get_ipc_response(
            &window,
            tauri::webview::InvokeRequest {
                cmd: "get_startup_state".into(),
                callback: tauri::ipc::CallbackFn(0),
                error: tauri::ipc::CallbackFn(1),
                url: "http://tauri.localhost".parse()?,
                body: tauri::ipc::InvokeBody::Json(serde_json::json!({})),
                headers: Default::default(),
                invoke_key: tauri::test::INVOKE_KEY.into(),
            },
        )
        .map_err(|error| format!("Early startup IPC rejected: {error:?}"))?
        .deserialize::<serde_json::Value>()?;
        assert_eq!(response["stage"], "preparing");
        assert_eq!(response["backendReady"], false);
        Ok(())
    }
}
