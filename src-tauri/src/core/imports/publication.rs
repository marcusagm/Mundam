//! Ledger input and receipt for a sealed item prepared by the host.
use super::ImportItem;
use uuid::Uuid;

/// Identifies the authority that must still be valid when the Ledger publishes.
#[derive(Debug, Clone)]
pub enum PublicationAuthority {
    /// An action initiated by the trusted application.
    Host,
    /// An action submitted by a previously authenticated paired importer.
    Paired(PublicationConsent),
}

/// A server-resolved consent version. Carries no bearer or verifier.
#[derive(Debug, Clone)]
pub struct PublicationConsent {
    pub(crate) client_id: String,
    pub(crate) catalog_id: String,
    pub(crate) grant_version: u64,
    pub(crate) server_key: String,
}

/// Stable key that links a staging claim to its catalog commit proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportPublicationAttempt {
    pub batch_id: Uuid,
    pub item_id: String,
    pub attempt_id: Uuid,
}

/// An internal command assembled after staging and authorization.
#[derive(Debug, Clone)]
pub struct PublishPreparedImport {
    /// Who requested publication and which consent must be checked.
    pub authority: PublicationAuthority,
    /// Identifier of the durable staging claim, when publishing from staging.
    pub attempt_id: Option<Uuid>,
    /// Durable staging batch identity.
    pub batch_id: Uuid,
    /// Opaque host-generated name of the temporary file.
    pub prepared_id: Uuid,
    /// Indexed destination resolved before staging.
    pub folder_id: String,
    /// Validated item declaration with its effective tags.
    pub item: ImportItem,
    /// SHA-256 computed by the host across the complete sealed upload.
    pub sha256: [u8; 32],
}

/// Immutable result returned even when the asset has since been renamed or removed.
#[derive(Debug, PartialEq, Eq)]
pub struct PublishedImport {
    /// Catalog identity assigned at publication.
    pub asset_id: String,
    /// Name allocated at publication, including any collision suffix.
    pub file_name: String,
}
