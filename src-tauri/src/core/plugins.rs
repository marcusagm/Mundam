//! Profile-scoped publisher trust and package admission contracts.
//!
//! Store mutations must keep publisher trust and installed package bytes consistent,
//! including during retries and revocation by another process.

use crate::core::AppResult;
use async_trait::async_trait;
use mundam_plugin_sdk::{Permission, Target};

pub mod asset_data;
pub mod inspector;
pub mod presentation;

/// Explicitly approved publisher identity from an independent source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublisherAnchor {
    pub namespace: String,
    pub fingerprint: [u8; 32],
}

/// Recorded identity and its current trust state.
#[derive(Debug)]
pub struct RegisteredPublisher {
    pub anchor: PublisherAnchor,
    pub revoked: bool,
}

/// Verified metadata sufficient to review an installation request.
#[derive(Debug)]
pub struct PackagePreview {
    pub publisher: PublisherAnchor,
    pub package_digest: [u8; 32],
    pub plugin_id: String,
    pub version: String,
    pub name: String,
    pub description: String,
    pub license: String,
    pub keywords: Vec<String>,
    pub targets: Vec<Target>,
    pub icon_png: Vec<u8>,
    pub required_permissions: Vec<Permission>,
    pub optional_permissions: Vec<Permission>,
    pub compatible: bool,
}

/// The user approved these exact verified package bytes for installation.
#[derive(Clone, Copy, Debug)]
pub struct InstallApproval {
    pub package_digest: [u8; 32],
}

/// Small inventory projection that does not load package contents.
#[derive(Debug, PartialEq, Eq)]
pub struct InstalledPlugin {
    pub id: String,
    pub publisher: String,
    pub version: String,
    pub name: String,
    pub enabled: bool,
    pub approved: bool,
    pub package_revision: String,
    pub rollback_version: Option<String>,
}

/// Current package identity and bytes used to plan a profile-wide replacement.
pub struct StoredPluginPackage {
    pub version: String,
    pub package_digest: [u8; 32],
    pub package_revision: String,
    pub publisher: PublisherAnchor,
    pub bytes: Vec<u8>,
}

/// The only supported replacement directions for a declarative package.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginReplacementKind {
    Update,
    Rollback,
}

/// A replacement review bound to the installed snapshot.
#[derive(Clone, Debug)]
pub struct PluginReplacement {
    pub kind: PluginReplacementKind,
    pub current_version: String,
    pub current_digest: [u8; 32],
    pub current_revision: String,
    pub added_permissions: Vec<Permission>,
    pub removed_permissions: Vec<Permission>,
}

/// Verified package data and the operation offered for consent.
#[derive(Debug)]
pub struct PluginPackageChange {
    pub preview: PackagePreview,
    pub replacement: Option<PluginReplacement>,
}

/// A verified package paired with the metadata rechecked at commit.
pub struct PluginInstallRecord<'a> {
    pub preview: &'a PackagePreview,
    pub bytes: &'a [u8],
}

/// Permissions reviewed for one installed package in the active catalog.
#[derive(Debug)]
pub struct PluginActivationReview {
    pub plugin_id: String,
    pub catalog_id: String,
    pub name: String,
    pub version: String,
    pub package_digest: [u8; 32],
    pub package_revision: String,
    pub required_permissions: Vec<Permission>,
    pub optional_permissions: Vec<Permission>,
}

/// Atomic operations on the local profile, independent of the selected asset catalog.
#[async_trait]
pub trait PluginRegistry: Send + Sync {
    /// Records a separately confirmed publisher. Identical active approvals may be replayed.
    async fn trust_publisher(&self, anchor: &PublisherAnchor) -> AppResult<()>;

    /// Returns only active trust. The fingerprint supplied by a package is a lookup hint.
    async fn active_publisher(&self, fingerprint: [u8; 32]) -> AppResult<Option<PublisherAnchor>>;

    /// Lists registered identities, including revoked ones.
    async fn list_publishers(&self) -> AppResult<Vec<RegisteredPublisher>>;

    /// Retains the revocation and disables affected installed packages in one commit.
    async fn revoke_publisher(&self, anchor: &PublisherAnchor) -> AppResult<()>;

    /// Rechecks trust and installs the verified bytes as disabled, or replays an identical install.
    async fn install(&self, record: PluginInstallRecord<'_>) -> AppResult<InstalledPlugin>;

    /// Lists inventory and active state for a catalog, without reading package BLOBs.
    async fn list_installed(&self, catalog_id: &str) -> AppResult<Vec<InstalledPlugin>>;

    /// Loads a snapshot of trusted, currently approved packages for the catalog.
    async fn approved_packages(
        &self,
        catalog_id: &str,
    ) -> AppResult<Vec<presentation::ApprovedPluginPackage>>;

    /// Holds the current approval stable until the caller finishes its catalog operation.
    async fn lock_approved_package(
        &self,
        catalog_id: &str,
        plugin_id: &str,
    ) -> AppResult<Option<Box<dyn presentation::LockedPluginPackage>>>;

    /// Loads the current package and its admission identity.
    async fn current_package(&self, plugin_id: &str) -> AppResult<Option<StoredPluginPackage>>;

    /// Loads the one retained rollback candidate and its recorded digest.
    async fn rollback_package(&self, plugin_id: &str) -> AppResult<Option<(Vec<u8>, [u8; 32])>>;

    /// Swaps an unchanged current package and revokes all grants in the same commit.
    async fn replace(
        &self,
        record: PluginInstallRecord<'_>,
        replacement: &PluginReplacement,
    ) -> AppResult<InstalledPlugin>;

    /// Removes exactly the reviewed current version and its dependent profile state.
    async fn remove(&self, plugin_id: &str, current_revision: &str) -> AppResult<()>;

    /// Commits consent only while the reviewed package and publisher are still current.
    async fn enable(
        &self,
        review: &PluginActivationReview,
        permissions: &[Permission],
    ) -> AppResult<()>;

    /// Revokes a plugin's consent for the given catalog.
    async fn disable(&self, catalog_id: &str, plugin_id: &str) -> AppResult<()>;
}
