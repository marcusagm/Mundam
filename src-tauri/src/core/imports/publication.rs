//! Ledger input and receipt for a sealed item prepared by the host.
use super::ImportItem;
use uuid::Uuid;

/// An internal command assembled after staging and authorization.
#[derive(Debug, Clone)]
pub struct PublishPreparedImport {
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
