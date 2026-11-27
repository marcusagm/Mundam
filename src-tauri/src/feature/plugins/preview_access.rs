//! Verifies the native image artifact and mediates reads within one host-admitted attempt.

use crate::core::{
    plugins::{
        derived_data::{PluginDerivedDataLedger, PluginPreviewRevision},
        preview_access::{
            HeldPluginPreviewApproval, PluginPreviewApproval, PluginPreviewStorage,
            MAX_PLUGIN_PREVIEW_BYTES, MAX_PLUGIN_PREVIEW_CHUNK,
        },
        PluginRegistry,
    },
    AppError, AppResult,
};
use mundam_plugin_sdk::Permission;
use std::{future::Future, ops::Range, sync::Arc, time::Duration};
use tokio::time::{sleep_until, Instant};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use super::{inspector::verify_approved_package, PluginService};

const MAX_LEASE_TIME: Duration = Duration::from_secs(15);
const MAX_READ_CALLS: usize = 32;
const MAX_DELIVERED_BYTES: usize = MAX_PLUGIN_PREVIEW_BYTES;

/// Size and opaque handle available to an attempt's transport.
#[derive(Clone, Copy)]
pub struct PluginPreviewOffer {
    pub handle: Uuid,
    pub byte_count: usize,
}

/// The host keeps this value inside the owning attempt. A handle alone cannot look up a lease.
struct PluginPreviewLease {
    offer: PluginPreviewOffer,
    ledger: Arc<dyn PluginDerivedDataLedger>,
    asset_id: String,
    revision: PluginPreviewRevision,
    deadline: Instant,
    cancellation: CancellationToken,
    bytes: Vec<u8>,
    read_calls: usize,
    delivered_bytes: usize,
}

/// Identities and lifetime resolved by the host before admitting a worker attempt.
pub struct PluginPreviewAttempt {
    pub plugin_id: String,
    pub worker_id: String,
    pub package_digest: [u8; 32],
    pub selected_asset_id: String,
    pub preview_revision: PluginPreviewRevision,
    pub deadline: Instant,
    pub cancellation: CancellationToken,
}

/// Verified worker bytes paired with the preview capability from the same approval.
pub struct ApprovedPluginWorker {
    module: Vec<u8>,
    preview: ApprovedPluginPreview,
}

impl ApprovedPluginWorker {
    /// Rechecks the approval and source at the point where the supervisor admits a launch.
    ///
    /// # Errors
    /// Rejects a revoked approval, stale publication or expired attempt.
    pub async fn admit_launch(self) -> AppResult<(Vec<u8>, ApprovedPluginPreview)> {
        let _approval = self.preview.lock_approval().await?;
        self.preview.lease.revalidate_source().await?;
        Ok((self.module, self.preview))
    }
}

/// Preview capability scoped to one worker attempt and one exact catalog approval.
pub struct ApprovedPluginPreview {
    registry: Arc<dyn PluginRegistry>,
    approval: PluginPreviewApproval,
    lease: PluginPreviewLease,
}

impl PluginService {
    /// Binds a worker's preview to the signed package, current consent and host selection.
    ///
    /// # Errors
    /// Rejects safe mode, missing consent, invalid or incompatible packages and
    /// unavailable, modified, expired or cancelled previews.
    pub async fn prepare_signed_worker(
        &self,
        ledger: Arc<dyn PluginDerivedDataLedger>,
        storage: &dyn PluginPreviewStorage,
        attempt: PluginPreviewAttempt,
    ) -> AppResult<ApprovedPluginWorker> {
        self.require_activation_available()?;
        let deadline = attempt.deadline.min(Instant::now() + MAX_LEASE_TIME);
        let (approval, module) = within_attempt(deadline, &attempt.cancellation, async {
            let catalog_id = ledger.plugin_catalog_id().await?;
            let executor = self.executor.ok_or_else(preview_denied)?;
            let api_version = self.api_version.clone();
            let locked = self
                .registry
                .lock_approved_package(&catalog_id, &attempt.plugin_id)
                .await?
                .ok_or_else(preview_denied)?;
            let expected_digest = attempt.package_digest;
            let worker_id = attempt.worker_id.clone();
            tokio::task::spawn_blocking(move || {
                let package = locked.package();
                if package.digest != expected_digest || package.activation_epoch.is_empty() {
                    return Err(preview_denied());
                }
                let verified = verify_approved_package(package, executor, &api_version)?;
                let manifest = verified.manifest().manifest();
                let module = match (&manifest.worker, verified.worker_module()) {
                    (Some(worker), Some(module)) if worker.id == worker_id => module,
                    _ => return Err(preview_denied()),
                };
                if [Permission::SelectionRead, Permission::AssetPreviewRead]
                    .iter()
                    .any(|permission| {
                        !package.permissions.contains(permission)
                            || !(manifest.permissions.contains(permission)
                                || manifest.optional_permissions.contains(permission))
                    })
                {
                    return Err(preview_denied());
                }
                let approval = PluginPreviewApproval {
                    catalog_id,
                    plugin_id: package.id.clone(),
                    publisher_fingerprint: package.publisher.fingerprint,
                    package_digest: package.digest,
                    activation_epoch: package.activation_epoch.clone(),
                };
                Ok((approval, module.to_vec()))
            })
            .await
            .map_err(|_| AppError::Internal("Plugin preview verification task failed".into()))?
        })
        .await?;
        let lease = prepare_image_preview(
            ledger,
            storage,
            attempt.selected_asset_id,
            attempt.preview_revision,
            deadline,
            attempt.cancellation,
        )
        .await?;
        let preview = ApprovedPluginPreview {
            registry: Arc::clone(&self.registry),
            approval,
            lease,
        };
        let _approval = preview.lock_approval().await?;
        Ok(ApprovedPluginWorker { module, preview })
    }
}

impl ApprovedPluginPreview {
    /// Announces the handle and exact snapshot size without exposing its disk location.
    pub fn offer(&self) -> PluginPreviewOffer {
        self.lease.offer
    }

    /// Reads a bounded range while its original catalog approval remains active.
    ///
    /// # Errors
    /// Rejects revoked consent, stale publications, invalid requests and expired attempts.
    pub async fn read(&mut self, handle: Uuid, offset: usize, length: usize) -> AppResult<&[u8]> {
        let range = self.lease.reserve_read(handle, offset, length)?;
        let _approval = self.lock_approval().await?;
        self.lease.read_range(range).await
    }

    async fn lock_approval(&self) -> AppResult<Box<dyn HeldPluginPreviewApproval>> {
        within_attempt(self.lease.deadline, &self.lease.cancellation, async {
            self.registry
                .lock_preview_approval(&self.approval)
                .await?
                .ok_or_else(preview_denied)
        })
        .await
    }
}

async fn prepare_image_preview(
    ledger: Arc<dyn PluginDerivedDataLedger>,
    storage: &dyn PluginPreviewStorage,
    asset_id: String,
    revision: PluginPreviewRevision,
    attempt_deadline: Instant,
    cancellation: CancellationToken,
) -> AppResult<PluginPreviewLease> {
    let deadline = attempt_deadline.min(Instant::now() + MAX_LEASE_TIME);
    let bytes = within_attempt(
        deadline,
        &cancellation,
        verified_bytes(ledger.as_ref(), storage, &asset_id, revision),
    )
    .await?;
    Ok(PluginPreviewLease {
        offer: PluginPreviewOffer {
            handle: Uuid::new_v4(),
            byte_count: bytes.len(),
        },
        ledger,
        asset_id,
        revision,
        deadline,
        cancellation,
        bytes,
        read_calls: 0,
        delivered_bytes: 0,
    })
}

impl PluginPreviewLease {
    #[cfg(test)]
    fn offer(&self) -> PluginPreviewOffer {
        self.offer
    }

    #[cfg(test)]
    async fn read(&mut self, handle: Uuid, offset: usize, length: usize) -> AppResult<&[u8]> {
        let range = self.reserve_read(handle, offset, length)?;
        self.read_range(range).await
    }

    fn reserve_read(
        &mut self,
        handle: Uuid,
        offset: usize,
        length: usize,
    ) -> AppResult<Range<usize>> {
        ensure_active(self.deadline, &self.cancellation)?;
        if self.read_calls >= MAX_READ_CALLS {
            return Err(AppError::Forbidden(
                "Plugin preview read budget exhausted".into(),
            ));
        }
        self.read_calls += 1;
        if self.offer.handle != handle {
            return Err(AppError::Forbidden(
                "Unknown preview handle in this attempt".into(),
            ));
        }
        let end = offset
            .checked_add(length)
            .filter(|&end| end <= self.bytes.len())
            .filter(|_| (1..=MAX_PLUGIN_PREVIEW_CHUNK).contains(&length))
            .ok_or_else(|| AppError::ValidationFailed("Invalid plugin preview range".into()))?;
        if self.delivered_bytes + length > MAX_DELIVERED_BYTES {
            return Err(AppError::Forbidden(
                "Plugin preview read budget exhausted".into(),
            ));
        }
        Ok(offset..end)
    }

    async fn read_range(&mut self, range: Range<usize>) -> AppResult<&[u8]> {
        self.revalidate_source().await?;
        self.delivered_bytes += range.len();
        Ok(&self.bytes[range])
    }

    async fn revalidate_source(&self) -> AppResult<()> {
        within_attempt(
            self.deadline,
            &self.cancellation,
            ensure_current(self.ledger.as_ref(), &self.asset_id, self.revision),
        )
        .await
    }
}

async fn verified_bytes(
    ledger: &dyn PluginDerivedDataLedger,
    storage: &dyn PluginPreviewStorage,
    asset_id: &str,
    revision: PluginPreviewRevision,
) -> AppResult<Vec<u8>> {
    let artifact = ledger
        .plugin_preview_artifact(asset_id)
        .await?
        .filter(|artifact| artifact.source.revision == revision)
        .ok_or_else(missing_preview)?;
    if !artifact.family.eq_ignore_ascii_case("IMAGE") {
        return Err(AppError::FormatNotSupported("Plugin image preview".into()));
    }
    if artifact.file_name.split_once('.').map(|(owner, _)| owner) != Some(asset_id) {
        return Err(AppError::InvalidProcessingOutput);
    }
    let bytes = storage.read_thumbnail(&artifact.file_name).await?;
    if bytes.len() > MAX_PLUGIN_PREVIEW_BYTES
        || !bytes.starts_with(b"RIFF")
        || bytes.get(8..12) != Some(b"WEBP")
        || blake3::hash(&bytes).as_bytes() != &artifact.source.content_digest
    {
        return Err(AppError::InvalidProcessingOutput);
    }
    ensure_current(ledger, asset_id, revision).await?;
    Ok(bytes)
}

async fn ensure_current(
    ledger: &dyn PluginDerivedDataLedger,
    asset_id: &str,
    revision: PluginPreviewRevision,
) -> AppResult<()> {
    ledger
        .plugin_preview_artifact(asset_id)
        .await?
        .filter(|artifact| {
            artifact.source.revision == revision && artifact.family.eq_ignore_ascii_case("IMAGE")
        })
        .map(|_| ())
        .ok_or_else(missing_preview)
}

fn missing_preview() -> AppError {
    AppError::NotFound("Current plugin preview".into())
}

fn preview_denied() -> AppError {
    AppError::Forbidden("Plugin preview permission is not active".into())
}

async fn within_attempt<T>(
    deadline: Instant,
    cancellation: &CancellationToken,
    operation: impl Future<Output = AppResult<T>>,
) -> AppResult<T> {
    ensure_active(deadline, cancellation)?;
    let value = tokio::select! {
        biased;
        _ = cancellation.cancelled() => return Err(AppError::Cancelled),
        _ = sleep_until(deadline) => return Err(AppError::Cancelled),
        result = operation => result?,
    };
    ensure_active(deadline, cancellation)?;
    Ok(value)
}

fn ensure_active(deadline: Instant, cancellation: &CancellationToken) -> AppResult<()> {
    if cancellation.is_cancelled() || Instant::now() >= deadline {
        return Err(AppError::Cancelled);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
