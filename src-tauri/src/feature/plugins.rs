//! Administrative package review and exact-byte installation.

use crate::core::{
    plugins::{
        InstallApproval, InstalledPlugin, PackagePreview, PluginActivationReview,
        PluginInstallRecord, PluginRegistry, PublisherAnchor, RegisteredPublisher,
    },
    AppError, AppResult,
};
use mundam_plugin_sdk::{
    package::{package_key_hint, verify_package, TrustedPublisher, VerifiedPackage},
    valid_namespace, Executor, Permission, Version,
};
use std::{collections::HashSet, sync::Arc};

mod inspector;
mod package_changes;

/// Application entry point for a local desktop adapter or an independent host.
pub struct PluginService {
    registry: Arc<dyn PluginRegistry>,
    executor: Option<Executor>,
    api_version: Version,
    safe_mode: bool,
}

impl PluginService {
    /// Uses the actual process architecture and the supported declarative API.
    pub fn new(registry: Arc<dyn PluginRegistry>) -> Self {
        Self {
            registry,
            executor: Executor::from_system(std::env::consts::OS, std::env::consts::ARCH),
            api_version: Version::new(1, 0, 0),
            safe_mode: false,
        }
    }

    /// Applies the host's startup policy before exposing enabled plugins.
    pub fn with_safe_mode(mut self, safe_mode: bool) -> Self {
        self.safe_mode = safe_mode;
        self
    }

    /// Reports whether the host suppresses plugin activation for this session.
    pub fn safe_mode(&self) -> bool {
        self.safe_mode
    }

    /// Records an identity entered and confirmed separately from package selection.
    ///
    /// # Errors
    /// Rejects malformed namespaces, conflicts with recorded identities and storage failures.
    pub async fn trust_publisher(&self, anchor: PublisherAnchor) -> AppResult<()> {
        if !valid_namespace(&anchor.namespace) {
            return Err(AppError::ValidationFailed(
                "Invalid plugin publisher namespace".into(),
            ));
        }
        self.registry.trust_publisher(&anchor).await
    }

    /// Revokes an exact publisher identity and disables its installed plugins.
    ///
    /// # Errors
    /// Returns an error if the anchor is unknown or the storage update fails.
    pub async fn revoke_publisher(&self, anchor: PublisherAnchor) -> AppResult<()> {
        self.registry.revoke_publisher(&anchor).await
    }

    /// Verifies the package and returns the information needed for a consent screen.
    ///
    /// # Errors
    /// Rejects malformed packages, untrusted publishers and signature or content failures.
    pub async fn preview(&self, bytes: Vec<u8>) -> AppResult<PackagePreview> {
        let (preview, _) = self.verify(bytes).await?;
        Ok(preview)
    }

    /// Installs the bytes reviewed by the user without enabling the plugin or granting access.
    ///
    /// # Errors
    /// Rejects changed bytes, a revoked publisher, incompatible targets, conflicting
    /// installed versions and storage failures.
    pub async fn install(
        &self,
        bytes: Vec<u8>,
        approval: InstallApproval,
    ) -> AppResult<InstalledPlugin> {
        let (preview, bytes) = self.verify(bytes).await?;
        if preview.package_digest != approval.package_digest {
            return Err(AppError::ValidationFailed(
                "Plugin package changed after review".into(),
            ));
        }
        if !preview.compatible {
            return Err(AppError::ValidationFailed(
                "Plugin package is incompatible with this host".into(),
            ));
        }
        self.registry
            .install(PluginInstallRecord {
                preview: &preview,
                bytes: &bytes,
            })
            .await
    }

    /// Returns a bounded-metadata inventory without opening installed plugin packages.
    ///
    /// # Errors
    /// Returns storage failures.
    pub async fn list_installed(&self, catalog_id: &str) -> AppResult<Vec<InstalledPlugin>> {
        let mut installed = self.registry.list_installed(catalog_id).await?;
        if self.safe_mode {
            for plugin in &mut installed {
                plugin.enabled = false;
            }
        }
        Ok(installed)
    }

    /// Rechecks the installed package before requesting catalog-specific approval.
    ///
    /// # Errors
    /// Rejects safe mode, untrusted or incompatible packages and storage failures.
    pub async fn review_activation(
        &self,
        catalog_id: String,
        plugin_id: &str,
    ) -> AppResult<PluginActivationReview> {
        self.require_activation_available()?;
        let current = self.require_current_package(plugin_id).await?;
        let preview = self.preview(current.bytes).await?;
        if preview.plugin_id != plugin_id
            || preview.package_digest != current.package_digest
            || preview.publisher != current.publisher
            || !preview.compatible
        {
            return Err(AppError::ValidationFailed(
                "Plugin package is incompatible with this host".into(),
            ));
        }
        Ok(PluginActivationReview {
            plugin_id: preview.plugin_id,
            catalog_id,
            name: preview.name,
            version: preview.version,
            package_digest: preview.package_digest,
            package_revision: current.package_revision,
            required_permissions: preview.required_permissions,
            optional_permissions: preview.optional_permissions,
        })
    }

    /// Records every required permission and only the optional permissions selected by the user.
    ///
    /// # Errors
    /// Rejects safe mode, duplicates, undeclared permissions, changed packages and revoked publishers.
    pub async fn enable(
        &self,
        review: &PluginActivationReview,
        selected_optional: Vec<Permission>,
    ) -> AppResult<()> {
        self.require_activation_available()?;
        let current = self
            .review_activation(review.catalog_id.clone(), &review.plugin_id)
            .await?;
        if current.package_digest != review.package_digest
            || current.package_revision != review.package_revision
        {
            return Err(AppError::ValidationFailed(
                "Plugin package changed after permission review".into(),
            ));
        }
        let selected: HashSet<_> = selected_optional.iter().copied().collect();
        if selected.len() != selected_optional.len()
            || !selected
                .iter()
                .all(|permission| current.optional_permissions.contains(permission))
        {
            return Err(AppError::ValidationFailed(
                "Plugin requested unreviewed permissions".into(),
            ));
        }
        let permissions: Vec<_> = current
            .required_permissions
            .iter()
            .copied()
            .chain(
                current
                    .optional_permissions
                    .iter()
                    .copied()
                    .filter(|permission| selected.contains(permission)),
            )
            .collect();
        self.registry.enable(&current, &permissions).await
    }

    /// Revokes approval for one catalog, including during a safe-mode session.
    ///
    /// # Errors
    /// Returns storage failures.
    pub async fn disable(&self, catalog_id: &str, plugin_id: &str) -> AppResult<()> {
        self.registry.disable(catalog_id, plugin_id).await
    }

    fn require_activation_available(&self) -> AppResult<()> {
        if self.safe_mode {
            return Err(AppError::Forbidden(
                "Plugin activation is unavailable in safe mode".into(),
            ));
        }
        Ok(())
    }

    /// Lists publisher identities and their revocation state for local administration.
    ///
    /// # Errors
    /// Returns storage failures.
    pub async fn list_publishers(&self) -> AppResult<Vec<RegisteredPublisher>> {
        self.registry.list_publishers().await
    }

    async fn verify(&self, bytes: Vec<u8>) -> AppResult<(PackagePreview, Vec<u8>)> {
        let fingerprint = package_key_hint(&bytes)
            .map_err(|_| AppError::ValidationFailed("Invalid plugin package envelope".into()))?;
        let publisher = self
            .registry
            .active_publisher(fingerprint)
            .await?
            .ok_or_else(|| AppError::Forbidden("Plugin publisher key is not trusted".into()))?;
        let executor = self.executor;
        let api_version = self.api_version.clone();

        tokio::task::spawn_blocking(move || {
            let verified = check_signed_package(&bytes, &publisher)?;
            let declaration = verified.manifest().manifest();
            let preview = PackagePreview {
                publisher,
                package_digest: verified.package_digest(),
                plugin_id: declaration.id.clone(),
                version: declaration.version.clone(),
                name: declaration.name.clone(),
                description: declaration.description.clone(),
                license: declaration.license.clone(),
                keywords: declaration.keywords.clone(),
                targets: declaration.targets.clone(),
                icon_png: verified.icon().to_vec(),
                required_permissions: declaration.permissions.clone(),
                optional_permissions: declaration.optional_permissions.clone(),
                compatible: executor
                    .is_some_and(|target| verified.manifest().supports(target, &api_version)),
            };
            Ok((preview, bytes))
        })
        .await
        .map_err(|_| AppError::Internal("Plugin verification task failed".into()))?
    }
}

fn check_signed_package<'a>(
    bytes: &'a [u8],
    publisher: &PublisherAnchor,
) -> AppResult<VerifiedPackage<'a>> {
    verify_package(
        bytes,
        &TrustedPublisher {
            namespace: &publisher.namespace,
            key_fingerprint: publisher.fingerprint,
        },
    )
    .map_err(|_| {
        AppError::ValidationFailed("Plugin signature, publisher or content is invalid".into())
    })
}
