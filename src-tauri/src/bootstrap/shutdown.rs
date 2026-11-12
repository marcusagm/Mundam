//! Tauri adapter for application shutdown and its independent presentation.

use crate::feature::shutdown::{ShutdownService, ShutdownSnapshot};
use crate::lifecycle::LifecycleRegistry;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

fn publish(application: &AppHandle, snapshot: &ShutdownSnapshot) {
    if let Err(error) = application.emit_to("shutdown", "shutdown:changed", snapshot) {
        tracing::warn!(%error, "Unable to publish shutdown progress");
    }
}

fn create_window(application: &AppHandle) {
    let result = WebviewWindowBuilder::new(
        application,
        "shutdown",
        WebviewUrl::App("shutdown.html".into()),
    )
    .title("Mundam — Closing")
    .inner_size(480.0, 360.0)
    .center()
    .resizable(false)
    .decorations(false)
    .focused(false)
    .build();
    if let Err(error) = result {
        tracing::error!(%error, "Unable to create shutdown window");
        if let Some(window) = application.get_webview_window("main") {
            let _ = window.set_title("Mundam — Closing…");
        }
    }
}

async fn close_pool_if_idle(pool: &sqlx::SqlitePool) -> bool {
    let wait_started = std::time::Instant::now();
    let (connections, idle_connections) = loop {
        let connections = pool.size();
        let idle_connections = pool.num_idle();
        if idle_connections == connections as usize {
            break (connections, idle_connections);
        }
        if wait_started.elapsed() >= std::time::Duration::from_millis(250) {
            tracing::warn!(
                connections,
                idle_connections,
                elapsed_ms = wait_started.elapsed().as_millis() as u64,
                "SQLite pool: skipping explicit close because connections are not all idle"
            );
            return false;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    };
    let started = std::time::Instant::now();
    tracing::info!(
        connections,
        idle_connections,
        idle_wait_ms = wait_started.elapsed().as_millis() as u64,
        "SQLite pool: closing after registered tasks drained"
    );
    if tokio::time::timeout(std::time::Duration::from_secs(3), pool.close())
        .await
        .is_err()
    {
        tracing::warn!(
            elapsed_ms = started.elapsed().as_millis() as u64,
            "SQLite pool did not close within 3s"
        );
        false
    } else {
        tracing::info!(
            elapsed_ms = started.elapsed().as_millis() as u64,
            "SQLite pool: close completed"
        );
        true
    }
}

/// Claims and starts shutdown once; subsequent requests join the active operation.
pub fn request(application: &AppHandle) {
    let service = application.state::<ShutdownService>();
    match service.request() {
        Ok(true) => {}
        Ok(false) => return,
        Err(error) => {
            tracing::error!(%error, "Unable to initiate shutdown");
            return;
        }
    }
    tracing::info!("Application shutdown requested");
    let lifecycle = application
        .state::<Arc<LifecycleRegistry>>()
        .inner()
        .clone();
    let protocols = application
        .state::<Arc<crate::delivery::protocols::admission::ProtocolAdmission>>()
        .inner()
        .clone();
    let active = protocols.close();
    tracing::info!(active, "Asset protocol admission closed");
    lifecycle.root_token().cancel();

    let window_handle = application.clone();
    tauri::async_runtime::spawn(async move { create_window(&window_handle) });

    let handle = application.clone();
    tauri::async_runtime::spawn(async move {
        let service = handle.state::<ShutdownService>();
        match service.draining() {
            Ok(snapshot) => publish(&handle, &snapshot),
            Err(error) => tracing::error!(%error, "Unable to retain shutdown progress"),
        }
        let boot_clean = match handle.try_state::<crate::bootstrap::startup::StartupTask>() {
            Some(startup) => startup.wait().await,
            None => true,
        };
        let (tasks_clean, protocol_drain) = tokio::join!(
            lifecycle.shutdown_all(),
            tokio::time::timeout(std::time::Duration::from_secs(5), protocols.drain())
        );
        let protocol_clean = if protocol_drain.is_ok() {
            tracing::info!("Asset protocol handlers drained");
            true
        } else {
            tracing::warn!("Asset protocol handlers did not drain within 5s");
            false
        };
        let mut clean = tasks_clean && boot_clean && protocol_clean;
        if clean {
            if let Some(database) =
                handle.try_state::<Arc<crate::infra::database::manager::DbManager>>()
            {
                clean = close_pool_if_idle(database.pool()).await;
            } else {
                tracing::info!("SQLite pool: not initialized; no explicit close required");
            }
        } else {
            tracing::warn!("Skipping explicit database pool close after incomplete task shutdown");
        }
        tracing::info!(clean, "Application shutdown completed");
        match service.finish(!clean) {
            Ok(snapshot) => publish(&handle, &snapshot),
            Err(error) => tracing::error!(%error, "Unable to retain shutdown result"),
        }
        if let Err(error) = service.wait_for_presentation().await {
            tracing::error!(%error, "Unable to wait for shutdown presentation");
        }
        if let Err(error) = service.authorize_exit() {
            tracing::error!(%error, "Unable to authorize application exit");
            return;
        }
        handle.exit(0);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn checked_out_connection_prevents_explicit_pool_close(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;
        let connection = pool.acquire().await?;
        assert!(!close_pool_if_idle(&pool).await);
        assert!(!pool.is_closed());
        drop(connection);
        assert!(close_pool_if_idle(&pool).await);
        assert!(pool.is_closed());
        Ok(())
    }
}
