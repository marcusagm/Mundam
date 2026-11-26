//! Reviews local version changes before the registry compares and replaces the current snapshot.

use super::PluginService;
use crate::core::{
    plugins::{
        InstallApproval, InstalledPlugin, PackagePreview, PluginInstallRecord, PluginPackageChange,
        PluginReplacement, PluginReplacementKind, StoredPluginPackage,
    },
    AppError, AppResult,
};
use mundam_plugin_sdk::Version;
use std::cmp::Ordering;

impl PluginService {
    /// Plans a new installation or a forward update from a locally selected package.
    ///
    /// # Errors
    /// Rejects invalid packages, a different publisher, and non-increasing SemVer.
    pub async fn review_package_change(&self, bytes: Vec<u8>) -> AppResult<PluginPackageChange> {
        let preview = self.preview(bytes).await?;
        let Some(current) = self.registry.current_package(&preview.plugin_id).await? else {
            return Ok(PluginPackageChange {
                preview,
                replacement: None,
            });
        };
        if current.package_digest == preview.package_digest {
            return Ok(PluginPackageChange {
                preview,
                replacement: None,
            });
        }
        check_replacement(&preview, &current, PluginReplacementKind::Update)?;
        let installed = self.preview(current.bytes.clone()).await?;
        let replacement = describe_change(
            PluginReplacementKind::Update,
            current,
            Some(installed),
            &preview,
        );
        Ok(PluginPackageChange {
            preview,
            replacement: Some(replacement),
        })
    }

    /// Plans a rollback to the retained signed package.
    ///
    /// # Errors
    /// Rejects missing, corrupt, incompatible or revoked rollback packages.
    pub async fn review_rollback(
        &self,
        plugin_id: &str,
    ) -> AppResult<(PluginPackageChange, Vec<u8>)> {
        let current = self.require_current_package(plugin_id).await?;
        let (bytes, digest) = self
            .registry
            .rollback_package(plugin_id)
            .await?
            .ok_or_else(|| AppError::NotFound("No previous plugin version is available".into()))?;
        let (preview, bytes) = self.verify(bytes).await?;
        if preview.plugin_id != plugin_id || preview.package_digest != digest || !preview.compatible
        {
            return Err(AppError::ValidationFailed(
                "The previous plugin version is unavailable on this host".into(),
            ));
        }
        check_replacement(&preview, &current, PluginReplacementKind::Rollback)?;
        // The retained package can repair a damaged current package. Its own authenticated
        // permissions still appear in full when a comparison is unavailable.
        let installed = self.preview(current.bytes.clone()).await.ok();
        let replacement = describe_change(
            PluginReplacementKind::Rollback,
            current,
            installed,
            &preview,
        );
        Ok((
            PluginPackageChange {
                preview,
                replacement: Some(replacement),
            },
            bytes,
        ))
    }

    /// Revalidates the consented bytes and commits a guarded version change.
    ///
    /// # Errors
    /// Rejects changed bytes, stale consent, incompatible packages and revoked publishers.
    pub async fn replace_package(
        &self,
        bytes: Vec<u8>,
        approval: InstallApproval,
        replacement: &PluginReplacement,
    ) -> AppResult<InstalledPlugin> {
        let (preview, bytes) = self.verify(bytes).await?;
        if preview.package_digest != approval.package_digest || !preview.compatible {
            return Err(AppError::ValidationFailed(
                "The reviewed plugin package is unavailable on this host".into(),
            ));
        }
        let current = self.require_current_package(&preview.plugin_id).await?;
        if current.package_digest != replacement.current_digest
            || current.package_revision != replacement.current_revision
        {
            return Err(AppError::ValidationFailed(
                "The installed plugin changed after review".into(),
            ));
        }
        check_replacement(&preview, &current, replacement.kind)?;
        self.registry
            .replace(
                PluginInstallRecord {
                    preview: &preview,
                    bytes: &bytes,
                },
                replacement,
            )
            .await
    }

    /// Removes the exact package confirmed by the user, including revoked publishers.
    ///
    /// # Errors
    /// Rejects stale confirmations and storage failures.
    pub async fn remove_package(&self, plugin_id: &str, revision: &str) -> AppResult<()> {
        self.registry.remove(plugin_id, revision).await
    }

    pub(super) async fn require_current_package(
        &self,
        plugin_id: &str,
    ) -> AppResult<StoredPluginPackage> {
        self.registry
            .current_package(plugin_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Plugin is not installed".into()))
    }
}

fn check_replacement(
    candidate: &PackagePreview,
    current: &StoredPluginPackage,
    kind: PluginReplacementKind,
) -> AppResult<()> {
    if candidate.publisher != current.publisher {
        return Err(AppError::Forbidden(
            "The plugin publisher does not match the installed package".into(),
        ));
    }
    let candidate_version = Version::parse(&candidate.version)
        .map_err(|_| AppError::ValidationFailed("Invalid candidate plugin version".into()))?;
    let current_version = Version::parse(&current.version)
        .map_err(|_| AppError::Internal("Invalid installed plugin version".into()))?;
    let direction = match kind {
        PluginReplacementKind::Update => Ordering::Greater,
        PluginReplacementKind::Rollback => Ordering::Less,
    };
    if candidate_version.cmp_precedence(&current_version) != direction {
        return Err(AppError::ValidationFailed(
            "The package version cannot be used for this operation".into(),
        ));
    }
    Ok(())
}

fn describe_change(
    kind: PluginReplacementKind,
    current: StoredPluginPackage,
    installed: Option<PackagePreview>,
    candidate: &PackagePreview,
) -> PluginReplacement {
    let (mut added_permissions, mut removed_permissions) = (Vec::new(), Vec::new());
    if let Some(installed) = installed {
        added_permissions.extend(
            candidate
                .required_permissions
                .iter()
                .copied()
                .filter(|permission| !installed.required_permissions.contains(permission)),
        );
        added_permissions.extend(candidate.optional_permissions.iter().copied().filter(
            |permission| {
                !installed.required_permissions.contains(permission)
                    && !installed.optional_permissions.contains(permission)
            },
        ));
        removed_permissions.extend(
            installed
                .required_permissions
                .iter()
                .chain(&installed.optional_permissions)
                .copied()
                .filter(|permission| {
                    !candidate.required_permissions.contains(permission)
                        && !candidate.optional_permissions.contains(permission)
                }),
        );
    }
    PluginReplacement {
        kind,
        current_version: current.version,
        current_digest: current.package_digest,
        current_revision: current.package_revision,
        added_permissions,
        removed_permissions,
    }
}
