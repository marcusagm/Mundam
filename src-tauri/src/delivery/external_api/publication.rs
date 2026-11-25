//! Listener-owned publication with one admitted batch and durable per-item receipts.
use crate::core::{
    imports::{
        pairing::AuthenticatedImporter,
        publication::PublicationAuthority,
        staging::{UploadError, UploadItem, UploadOwner, UploadState},
    },
    ledger::port::TransactionalAssetLedger,
    AppError,
};
use crate::infra::database::import_staging::SqliteImportStaging;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::sync::{mpsc, watch};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Clone)]
pub(super) struct PublicationQueue {
    sender: mpsc::Sender<PublicationJob>,
    slot: Arc<PublicationSlot>,
    closing: CancellationToken,
}

struct PublicationSlot {
    batch: Mutex<Option<Uuid>>,
    idle: watch::Sender<bool>,
}

struct PublicationJob {
    batch: Uuid,
    client: Arc<AuthenticatedImporter>,
    slot: Arc<PublicationSlot>,
}

impl Drop for PublicationJob {
    fn drop(&mut self) {
        if let Ok(mut batch) = self.slot.batch.lock() {
            *batch = None;
            self.slot.idle.send_replace(true);
        }
    }
}

pub(super) enum Admission {
    Accepted,
    Busy,
    Closed,
}

pub(super) struct PublicationWorker {
    receiver: mpsc::Receiver<PublicationJob>,
    closing: CancellationToken,
    staging: SqliteImportStaging,
    app_data: PathBuf,
    ledger: Arc<dyn TransactionalAssetLedger>,
}

pub(super) fn channel(
    staging: SqliteImportStaging,
    app_data: PathBuf,
    ledger: Arc<dyn TransactionalAssetLedger>,
) -> (PublicationQueue, PublicationWorker) {
    let (sender, receiver) = mpsc::channel(1);
    let (idle, _) = watch::channel(true);
    let closing = CancellationToken::new();
    let queue = PublicationQueue {
        sender,
        slot: Arc::new(PublicationSlot {
            batch: Mutex::new(None),
            idle,
        }),
        closing: closing.clone(),
    };
    (
        queue,
        PublicationWorker {
            receiver,
            closing,
            staging,
            app_data,
            ledger,
        },
    )
}

impl PublicationQueue {
    pub(super) fn is_active(&self, batch: Uuid) -> bool {
        self.slot
            .batch
            .lock()
            .is_ok_and(|active| *active == Some(batch))
    }

    pub(super) fn schedule(&self, batch: Uuid, client: Arc<AuthenticatedImporter>) -> Admission {
        let Ok(mut active) = self.slot.batch.lock() else {
            return Admission::Closed;
        };
        if self.closing.is_cancelled() {
            return Admission::Closed;
        }
        if let Some(current) = *active {
            return if current == batch {
                Admission::Accepted
            } else {
                Admission::Busy
            };
        }
        let Ok(permit) = self.sender.try_reserve() else {
            return Admission::Closed;
        };
        *active = Some(batch);
        self.slot.idle.send_replace(false);
        permit.send(PublicationJob {
            batch,
            client,
            slot: self.slot.clone(),
        });
        Admission::Accepted
    }

    pub(super) fn close(&self) {
        let _active = self.slot.batch.lock();
        self.closing.cancel();
    }

    pub(super) async fn wait_idle(&self) {
        let mut idle = self.slot.idle.subscribe();
        let _ = idle.wait_for(|idle| *idle).await;
    }
}

pub(super) fn can_publish(item: &UploadItem) -> bool {
    item.state == UploadState::Ready || item.error.as_ref().is_some_and(|error| error.retryable)
}

impl PublicationWorker {
    pub(super) async fn run(mut self, queue: PublicationQueue, shutdown: CancellationToken) {
        loop {
            let job = tokio::select! {
                biased;
                () = shutdown.cancelled() => break,
                () = self.closing.cancelled() => break,
                job = self.receiver.recv() => job,
            };
            let Some(job) = job else { break };
            if let Err(error) = self.publish_batch(&job, &shutdown).await {
                tracing::warn!(batch = %job.batch, %error, "Import batch publication paused");
            }
        }
        queue.close();
        // Dropping queued jobs releases their admission slot while leaving staged bytes intact.
        drop(self.receiver);
    }

    async fn publish_batch(
        &self,
        job: &PublicationJob,
        shutdown: &CancellationToken,
    ) -> Result<(), UploadError> {
        let owner = UploadOwner {
            catalog_id: &job.client.catalog.destinations.catalog_id,
            client_id: &job.client.catalog.client_id,
        };
        let batch = job.batch.to_string();
        self.staging
            .reconcile_publication(&owner, job.batch, &self.app_data, self.ledger.as_ref())
            .await?;
        let receipt = self.staging.receipt(&owner, &batch, None, Some(1)).await?;
        for item in receipt.items.iter().filter(|item| can_publish(item)) {
            if self.closing.is_cancelled() || shutdown.is_cancelled() {
                break;
            }
            let result = self
                .staging
                .publish_item(
                    &owner,
                    job.batch,
                    &item.item_id,
                    PublicationAuthority::Paired(job.client.publication_consent()),
                    &self.app_data,
                    self.ledger.clone(),
                )
                .await;
            match result {
                Ok(_) | Err(UploadError::PublicationComplete) => {}
                Err(UploadError::Cancelled | UploadError::Expired | UploadError::Missing) => break,
                Err(error) => {
                    // A recoverable claim may still hold a native temporary. Settle it before
                    // recording the failure or admitting the next item.
                    self.staging
                        .reconcile_publication(
                            &owner,
                            job.batch,
                            &self.app_data,
                            self.ledger.as_ref(),
                        )
                        .await?;
                    let (code, retryable) = public_failure(&error);
                    self.staging
                        .record_publication_error(&owner, job.batch, &item.item_id, code, retryable)
                        .await?;
                    if retryable {
                        break;
                    }
                }
            }
        }
        Ok(())
    }
}

fn public_failure(error: &UploadError) -> (&'static str, bool) {
    match error {
        UploadError::Other(AppError::UnsupportedFormat(_)) => ("INVALID_MEDIA", false),
        UploadError::StagingCorrupt => ("STAGING_INTEGRITY_FAILED", false),
        UploadError::Other(AppError::Forbidden(_)) => ("PUBLICATION_AUTHORIZATION_CHANGED", true),
        UploadError::Other(AppError::ValidationFailed(_) | AppError::NotFound(_)) => {
            ("PUBLICATION_DESTINATION_OR_METADATA_UNAVAILABLE", true)
        }
        UploadError::DiskFull => ("PUBLICATION_STORAGE_FULL", true),
        _ => ("PUBLICATION_FAILED", true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::imports::{destinations::DestinationGrant, pairing::ImportScope};
    use crate::infra::{database::ledger::SqliteAssetLedger, events::TokioEventBus};
    use std::time::Duration;

    #[tokio::test]
    async fn publication_admission_coalesces_and_drains_the_active_slot(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let storage = tempfile::tempdir()?;
        let staging = SqliteImportStaging::open(&storage.path().join("staging.sqlite")).await?;
        let ledger = Arc::new(SqliteAssetLedger::new(
            sqlx::SqlitePool::connect("sqlite::memory:").await?,
            Arc::new(TokioEventBus::new()),
            storage.path().to_path_buf(),
            Arc::new(crate::core::path_guard::LedgerPathGuard::new()),
        ));
        let (queue, mut worker) = channel(staging, storage.path().to_path_buf(), ledger);
        let client = Arc::new(AuthenticatedImporter::new(
            "client".into(),
            1,
            DestinationGrant {
                catalog_id: "catalog".into(),
                root_ids: vec![],
            },
            vec![ImportScope::ImportAssets],
            "test".into(),
        ));
        let batch = Uuid::new_v4();
        assert!(matches!(
            queue.schedule(batch, client.clone()),
            Admission::Accepted
        ));
        assert!(matches!(
            queue.schedule(batch, client.clone()),
            Admission::Accepted
        ));
        assert!(matches!(
            queue.schedule(Uuid::new_v4(), client.clone()),
            Admission::Busy
        ));
        let job = worker.receiver.recv().await.ok_or("Missing accepted job")?;
        queue.close();
        assert!(matches!(queue.schedule(batch, client), Admission::Closed));
        assert!(
            tokio::time::timeout(Duration::from_millis(25), queue.wait_idle())
                .await
                .is_err()
        );
        drop(job);
        tokio::time::timeout(Duration::from_secs(1), queue.wait_idle()).await?;
        Ok(())
    }
}
