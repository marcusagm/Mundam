//! Owner for the explicitly activated Windows import listener.
#[cfg(windows)]
use super::listener::ImportHttpsListener;
#[cfg(windows)]
use crate::infra::database::import_staging::SqliteImportStaging;
#[cfg(windows)]
use crate::infra::import_identity;
use crate::{
    core::{ledger::port::TransactionalAssetLedger, AppError, AppResult},
    lifecycle::LifecycleRegistry,
};
#[cfg(windows)]
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Serialize;
use sqlx::SqlitePool;
use std::{path::Path, sync::Arc};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

const TASK_NAME: &str = "external_import_api";

/// Public trust information suitable for transfer over the trusted desktop channel.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportEndpoint {
    pub url: String,
    pub public_key_sha256: String,
    pub certificate_der: String,
}

struct Running {
    cancellation: CancellationToken,
    endpoint: ImportEndpoint,
    server_key: [u8; 32],
    publication: Option<super::publication::PublicationQueue>,
}

/// Serializes repeated start and stop requests for one desktop instance.
#[derive(Default)]
pub struct DesktopImportService {
    running: Mutex<Option<Running>>,
}

impl DesktopImportService {
    /// Returns the public endpoint and the running server's pairing identity.
    pub(crate) async fn pairing_session(&self) -> AppResult<(ImportEndpoint, [u8; 32])> {
        self.running
            .lock()
            .await
            .as_ref()
            .filter(|running| !running.cancellation.is_cancelled())
            .map(|running| (running.endpoint.clone(), running.server_key))
            .ok_or_else(|| AppError::Forbidden("The local import API is disabled".into()))
    }

    /// Returns the active endpoint or an absent endpoint while disabled.
    pub async fn endpoint(&self) -> Option<ImportEndpoint> {
        self.running
            .lock()
            .await
            .as_ref()
            .filter(|running| !running.cancellation.is_cancelled())
            .map(|running| running.endpoint.clone())
    }

    /// Loads the identity and starts a listener owned by the desktop lifecycle.
    ///
    /// # Errors
    /// Returns identity, socket and initialization failures, or cancellation during shutdown.
    #[cfg(windows)]
    pub async fn start(
        &self,
        lifecycle: &Arc<LifecycleRegistry>,
        pool: SqlitePool,
        directory: &Path,
        ledger: Arc<dyn TransactionalAssetLedger>,
    ) -> AppResult<ImportEndpoint> {
        let mut running = self.running.lock().await;
        if lifecycle.root_token().is_cancelled() {
            return Err(AppError::Cancelled);
        }
        if let Some(active) = running.as_ref() {
            if !active.cancellation.is_cancelled() {
                return Ok(active.endpoint.clone());
            }
            lifecycle.shutdown_by_name(TASK_NAME).await;
            running.take();
        }
        let identity_directory = directory.to_owned();
        let identity = tokio::task::spawn_blocking(move || {
            import_identity::load_or_create(&identity_directory)
        })
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
        let certificate_der = STANDARD.encode(identity.certificate.as_ref());
        let staging = SqliteImportStaging::open(
            &directory.join(crate::infra::database::import_staging::DATABASE_NAME),
        )
        .await?;
        let listener = ImportHttpsListener::bind_with_publication(
            pool,
            identity.certificate,
            identity.private_key.into(),
            staging,
            directory.to_path_buf(),
            ledger,
        )
        .await?;
        let queue = listener.publication_queue();
        let server_key = listener.server_key_digest();
        let endpoint = ImportEndpoint {
            url: format!("https://{}", listener.address()),
            public_key_sha256: blake3::Hash::from_bytes(server_key).to_hex().to_string(),
            certificate_der,
        };
        let publication_directory = directory.to_owned();
        let published_endpoint = endpoint.clone();
        let publication = tokio::task::spawn_blocking(move || {
            super::discovery::DiscoveryPublication::publish(
                &publication_directory,
                &published_endpoint.url,
                &published_endpoint.public_key_sha256,
            )
        })
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
        let cancellation = lifecycle.child_token();
        let worker = cancellation.clone();
        let owner = lifecycle.clone();
        if !lifecycle.spawn_unique(TASK_NAME.to_owned(), async move {
            let _finished = worker.clone().drop_guard();
            let _publication = publication;
            if let Err(error) = listener.serve(worker).await {
                tracing::error!(%error, "External import listener stopped");
                owner.report_failure(TASK_NAME);
            }
        }) {
            return Err(AppError::Cancelled);
        }
        *running = Some(Running {
            cancellation,
            endpoint: endpoint.clone(),
            server_key,
            publication: queue,
        });
        Ok(endpoint)
    }

    /// Reports that the platform's protected identity adapter is unavailable.
    ///
    /// # Errors
    /// Always returns a validation error until the platform adapter is implemented.
    #[cfg(not(windows))]
    pub async fn start(
        &self,
        _lifecycle: &Arc<LifecycleRegistry>,
        _pool: SqlitePool,
        _directory: &Path,
        _ledger: Arc<dyn TransactionalAssetLedger>,
    ) -> AppResult<ImportEndpoint> {
        Err(AppError::ValidationFailed(
            "The local import API is unavailable on this platform".into(),
        ))
    }

    /// Stops admission and waits for the registered listener to drain.
    pub async fn stop(&self, lifecycle: &LifecycleRegistry) {
        let mut running = self.running.lock().await;
        if let Some(active) = running.take() {
            if let Some(publication) = &active.publication {
                publication.close();
            }
            active.cancellation.cancel();
            if let Some(publication) = active.publication {
                publication.wait_idle().await;
            }
            lifecycle.shutdown_by_name(TASK_NAME).await;
        }
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use crate::infra::database::manager::DbManager;

    #[tokio::test]
    async fn external_import_desktop_service_is_explicit_stable_and_supervised() -> AppResult<()> {
        let directory = tempfile::tempdir()?;
        let database = DbManager::new(&directory.path().join("catalog.sqlite")).await?;
        let service = DesktopImportService::default();
        let lifecycle = Arc::new(LifecycleRegistry::new());
        let ledger = Arc::new(crate::infra::database::ledger::SqliteAssetLedger::new(
            database.pool().clone(),
            Arc::new(crate::infra::events::TokioEventBus::new()),
            directory.path().to_path_buf(),
            Arc::new(crate::core::path_guard::LedgerPathGuard::new()),
        ));
        assert!(service.endpoint().await.is_none());
        assert!(!directory.path().join("import-tls-identity.dpapi").exists());
        let first = service
            .start(
                &lifecycle,
                database.pool().clone(),
                directory.path(),
                ledger.clone(),
            )
            .await?;
        let discovery_path = directory.path().join(super::super::discovery::FILE_NAME);
        assert!(discovery_path.exists());
        let repeat = service
            .start(
                &lifecycle,
                database.pool().clone(),
                directory.path(),
                ledger.clone(),
            )
            .await?;
        assert_eq!(first.url, repeat.url);
        service.stop(&lifecycle).await;
        assert!(service.endpoint().await.is_none());
        assert!(!discovery_path.exists());
        let second = service
            .start(
                &lifecycle,
                database.pool().clone(),
                directory.path(),
                ledger.clone(),
            )
            .await?;
        assert_eq!(first.public_key_sha256, second.public_key_sha256);
        assert_eq!(first.certificate_der, second.certificate_der);
        assert!(lifecycle.shutdown_all().await);
        assert!(service.endpoint().await.is_none());
        assert!(!discovery_path.exists());
        assert!(matches!(
            service
                .start(
                    &lifecycle,
                    database.pool().clone(),
                    directory.path(),
                    ledger
                )
                .await,
            Err(AppError::Cancelled)
        ));
        database.close().await;
        Ok(())
    }
}
