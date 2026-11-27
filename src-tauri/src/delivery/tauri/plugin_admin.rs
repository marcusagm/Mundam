//! Desktop state for the profile registry and one short-lived package review.

use crate::{
    core::{
        plugins::{
            InstallApproval, InstalledPlugin, PluginActivationReview, PluginPackageChange,
            PluginReplacement, PublisherAnchor,
        },
        AppError, AppResult,
    },
    feature::plugins::PluginService,
    infra::database::plugins::SqlitePluginRegistry,
};
use mundam_plugin_sdk::{package::MAX_PACKAGE_BYTES, Permission};
use std::{path::Path, sync::Arc, time::Duration};
use tokio::{
    io::AsyncReadExt,
    sync::{Mutex, OnceCell},
    time::Instant,
};
use uuid::Uuid;

const REVIEW_LIFETIME: Duration = Duration::from_secs(300);

struct PendingReview {
    id: Uuid,
    expires_at: Instant,
    bytes: Vec<u8>,
    approval: InstallApproval,
    replacement: Option<PluginReplacement>,
}

struct PendingActivation {
    id: Uuid,
    expires_at: Instant,
    review: PluginActivationReview,
}

/// Opens the optional registry only when a settings command needs it.
#[derive(Default)]
pub(crate) struct DesktopPluginAdmin {
    service: OnceCell<PluginService>,
    pending: Mutex<Option<PendingReview>>,
    pending_activation: Mutex<Option<PendingActivation>>,
    pub(crate) selection: Mutex<()>,
    safe_mode: bool,
}

impl DesktopPluginAdmin {
    pub(crate) fn from_environment() -> Self {
        Self {
            safe_mode: std::env::var_os("MUNDAM_PLUGIN_SAFE_MODE").as_deref()
                == Some(std::ffi::OsStr::new("1")),
            ..Self::default()
        }
    }

    pub(crate) async fn service(&self, profile: &Path) -> AppResult<&PluginService> {
        self.service
            .get_or_try_init(|| async {
                let registry = SqlitePluginRegistry::open(&profile.join("plugins.sqlite")).await?;
                Ok(PluginService::new(Arc::new(registry)).with_safe_mode(self.safe_mode))
            })
            .await
    }

    pub(crate) async fn clear_review(&self) {
        self.pending.lock().await.take();
    }

    pub(crate) async fn review(
        &self,
        service: &PluginService,
        bytes: Vec<u8>,
    ) -> AppResult<(PluginPackageChange, Option<Uuid>)> {
        let change = service.review_package_change(bytes.clone()).await?;
        Ok(self.remember_review(change, bytes).await)
    }

    pub(crate) async fn review_rollback(
        &self,
        service: &PluginService,
        plugin_id: &str,
    ) -> AppResult<(PluginPackageChange, Option<Uuid>)> {
        let (change, bytes) = service.review_rollback(plugin_id).await?;
        Ok(self.remember_review(change, bytes).await)
    }

    async fn remember_review(
        &self,
        change: PluginPackageChange,
        bytes: Vec<u8>,
    ) -> (PluginPackageChange, Option<Uuid>) {
        let id = if change.preview.compatible {
            let id = Uuid::new_v4();
            *self.pending.lock().await = Some(PendingReview {
                id,
                expires_at: Instant::now() + REVIEW_LIFETIME,
                bytes,
                approval: InstallApproval {
                    package_digest: change.preview.package_digest,
                },
                replacement: change.replacement.clone(),
            });
            Some(id)
        } else {
            None
        };
        (change, id)
    }

    pub(crate) async fn discard(&self, id: Uuid) {
        {
            let mut pending = self.pending.lock().await;
            if pending.as_ref().is_some_and(|review| review.id == id) {
                pending.take();
            }
        }
        let mut activation = self.pending_activation.lock().await;
        if activation.as_ref().is_some_and(|review| review.id == id) {
            activation.take();
        }
    }

    pub(crate) async fn review_activation(
        &self,
        service: &PluginService,
        catalog_id: String,
        plugin_id: &str,
    ) -> AppResult<(
        Uuid,
        String,
        String,
        Vec<Permission>,
        Vec<Permission>,
        Option<String>,
    )> {
        let mut pending = self.pending_activation.lock().await;
        pending.take();
        let review = service.review_activation(catalog_id, plugin_id).await?;
        let id = Uuid::new_v4();
        let response = (
            id,
            review.name.clone(),
            review.version.clone(),
            review.required_permissions.clone(),
            review.optional_permissions.clone(),
            review.experimental_worker_id.clone(),
        );
        *pending = Some(PendingActivation {
            id,
            expires_at: Instant::now() + REVIEW_LIFETIME,
            review,
        });
        Ok(response)
    }

    pub(crate) async fn enable(
        &self,
        service: &PluginService,
        catalog_id: &str,
        id: Uuid,
        optional_permissions: Vec<Permission>,
    ) -> AppResult<()> {
        let approval = {
            let mut pending = self.pending_activation.lock().await;
            if !pending.as_ref().is_some_and(|approval| approval.id == id) {
                return Err(expired_activation());
            }
            pending.take().filter(|approval| {
                approval.review.catalog_id == catalog_id && Instant::now() < approval.expires_at
            })
        }
        .ok_or_else(expired_activation)?;
        service.enable(&approval.review, optional_permissions).await
    }

    pub(crate) async fn install(
        &self,
        service: &PluginService,
        id: Uuid,
    ) -> AppResult<InstalledPlugin> {
        let review = {
            let mut pending = self.pending.lock().await;
            if !pending.as_ref().is_some_and(|review| review.id == id) {
                return Err(expired_review());
            }
            pending
                .take()
                .filter(|review| Instant::now() < review.expires_at)
        }
        .ok_or_else(expired_review)?;
        if let Some(replacement) = review.replacement {
            service
                .replace_package(review.bytes, review.approval, &replacement)
                .await
        } else {
            service.install(review.bytes, review.approval).await
        }
    }
}

fn expired_review() -> AppError {
    AppError::ValidationFailed("This package review has expired. Choose the package again.".into())
}

fn expired_activation() -> AppError {
    AppError::ValidationFailed(
        "This permission review has expired. Review the plugin again.".into(),
    )
}

pub(crate) async fn read_selected_package(path: &Path) -> AppResult<Vec<u8>> {
    let valid_file = |metadata: &std::fs::Metadata| {
        metadata.is_file() && metadata.len() <= MAX_PACKAGE_BYTES as u64
    };
    if !valid_file(&tokio::fs::metadata(path).await?) {
        return Err(invalid_file());
    }
    let file = tokio::fs::File::open(path).await?;
    if !valid_file(&file.metadata().await?) {
        return Err(invalid_file());
    }
    let mut bytes = Vec::new();
    file.take((MAX_PACKAGE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .await?;
    if bytes.len() > MAX_PACKAGE_BYTES {
        return Err(invalid_file());
    }
    Ok(bytes)
}

fn invalid_file() -> AppError {
    AppError::ValidationFailed(
        "Choose a Mundam plugin package within the supported size limit.".into(),
    )
}

pub(crate) fn publisher_anchor(namespace: String, fingerprint: &str) -> AppResult<PublisherAnchor> {
    Ok(PublisherAnchor {
        namespace,
        fingerprint: parse_sha256(fingerprint)?,
    })
}

pub(crate) fn parse_sha256(fingerprint: &str) -> AppResult<[u8; 32]> {
    let invalid = || {
        AppError::ValidationFailed(
            "Enter a SHA-256 fingerprint with 64 hexadecimal characters.".into(),
        )
    };
    if fingerprint.len() != 64 || !fingerprint.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(invalid());
    }
    let mut bytes = [0; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&fingerprint[index * 2..index * 2 + 2], 16)
            .map_err(|_| invalid())?;
    }
    Ok(bytes)
}

pub(crate) fn fingerprint_hex(fingerprint: &[u8; 32]) -> String {
    fingerprint
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests;
