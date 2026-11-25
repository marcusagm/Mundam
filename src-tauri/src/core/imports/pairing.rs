//! Transport-independent consent and authentication for external import clients.
use super::{
    catalog::CatalogReadGrant, destinations::DestinationGrant, publication::PublicationConsent,
};
use crate::core::AppResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// The permissions the desktop can grant to a paired importer.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub enum ImportScope {
    #[serde(rename = "folders.read")]
    ReadFolders,
    #[serde(rename = "tags.read")]
    ReadTags,
    #[serde(rename = "assets.import")]
    ImportAssets,
    #[serde(rename = "tags.assignOnImport")]
    AssignTagsOnImport,
}

impl ImportScope {
    pub(crate) const ALL: [Self; 4] = [
        Self::ReadFolders,
        Self::ReadTags,
        Self::ImportAssets,
        Self::AssignTagsOnImport,
    ];

    pub(crate) fn mask(self) -> i64 {
        match self {
            Self::ReadFolders => 1,
            Self::ReadTags => 2,
            Self::ImportAssets => 4,
            Self::AssignTagsOnImport => 8,
        }
    }
}

/// A short-lived invitation returned to the trusted desktop.
pub struct PairingInvitation {
    pub code: String,
    pub expires_at: i64,
}

/// The requester keeps this secret until the desktop has approved the request.
pub struct PairingClaim {
    pub request_id: String,
    pub secret: String,
}

/// Information presented for explicit consent in the desktop.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingPairing {
    pub id: String,
    pub client_name: String,
    pub requested_scopes: Vec<ImportScope>,
}

/// Display information for a live paired client. Credentials are omitted.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairedImporter {
    pub id: String,
    pub client_name: String,
    pub scopes: Vec<ImportScope>,
    pub root_ids: Vec<String>,
}

/// The desktop's explicit decision about the requested permissions and library roots.
pub struct PairingApproval<'a> {
    pub catalog_id: &'a str,
    pub scopes: &'a [ImportScope],
    pub root_ids: &'a [String],
}

/// A bearer returned to the claiming client once. Keep it out of diagnostics.
pub struct IssuedCredential {
    pub client_id: String,
    pub bearer: String,
    pub expires_at: i64,
}

/// The server-resolved identity for an authenticated request.
pub struct AuthenticatedImporter {
    pub catalog: CatalogReadGrant,
    pub scopes: Vec<ImportScope>,
    publication_consent: PublicationConsent,
}

impl AuthenticatedImporter {
    /// Builds the corresponding catalog authorization from persisted grants.
    pub(crate) fn new(
        client_id: String,
        grant_version: u64,
        destinations: DestinationGrant,
        scopes: Vec<ImportScope>,
        server_key: String,
    ) -> Self {
        Self {
            publication_consent: PublicationConsent {
                client_id: client_id.clone(),
                catalog_id: destinations.catalog_id.clone(),
                grant_version,
                server_key,
            },
            catalog: CatalogReadGrant {
                client_id,
                grant_version,
                destinations,
                can_read_folders: scopes.contains(&ImportScope::ReadFolders),
                can_read_tags: scopes.contains(&ImportScope::ReadTags),
            },
            scopes,
        }
    }

    /// Captures the authority to revalidate inside the catalog writer.
    pub fn publication_consent(&self) -> PublicationConsent {
        self.publication_consent.clone()
    }
}

/// Trusted desktop administration and external authentication backed by the active catalog.
#[async_trait]
pub trait ImportPairing: Send + Sync {
    /// Opens a new single-use pairing invitation.
    async fn invite(&self) -> AppResult<PairingInvitation>;

    /// Claims an invitation for a named client and requests scopes for desktop approval.
    async fn request(
        &self,
        code: &str,
        name: &str,
        scopes: &[ImportScope],
    ) -> AppResult<PairingClaim>;

    /// Lists unexpired requests awaiting the user's decision.
    async fn pending(&self) -> AppResult<Vec<PendingPairing>>;

    /// Lists live paired clients for the current server identity.
    async fn clients(&self) -> AppResult<Vec<PairedImporter>>;

    /// Approves only requested scopes and explicitly selected roots from the current catalog.
    async fn approve(&self, id: &str, approval: PairingApproval<'_>) -> AppResult<()>;

    /// Rejects a pending request or cancels approval before the client redeems it.
    async fn reject(&self, id: &str) -> AppResult<()>;

    /// Consumes the approved claim and issues a private bearer.
    async fn redeem(&self, id: &str, secret: &str) -> AppResult<IssuedCredential>;

    /// Resolves current grants for an unexpired, unrevoked bearer.
    async fn authenticate(&self, bearer: &str) -> AppResult<AuthenticatedImporter>;

    /// Replaces a live bearer and invalidates the previous one and its cursors.
    async fn rotate(&self, bearer: &str) -> AppResult<IssuedCredential>;

    /// Revokes a client from the desktop in the currently open catalog.
    async fn revoke(&self, catalog_id: &str, client_id: &str) -> AppResult<()>;
}
