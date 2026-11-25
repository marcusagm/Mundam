//! Ledger input and receipt for a sealed item prepared by the host.
use super::ImportItem;
use crate::core::library::FileIdentity;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const VOLUME_PREFIX: &str = ".mundam-import-";
const VOLUME_SUFFIX: &str = ".part";

/// Derives the reserved destination-volume filename from a host attempt ID.
pub(crate) fn volume_temporary_name(id: Uuid) -> String {
    format!("{VOLUME_PREFIX}{id}{VOLUME_SUFFIX}")
}

/// Identifies only exact UUID-named destination temporaries in the Windows pilot.
pub(crate) fn is_volume_temporary(path: &Path) -> bool {
    if !cfg!(windows) {
        return false;
    }
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let name = name.to_ascii_lowercase();
    name.strip_prefix(VOLUME_PREFIX)
        .and_then(|name| name.strip_suffix(VOLUME_SUFFIX))
        .is_some_and(|id| id.len() == 36 && Uuid::parse_str(id).is_ok())
}

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

/// An authorized catalog folder and its verified physical identity.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ImportPreparationDestination {
    pub path: PathBuf,
    pub identity: FileIdentity,
}

/// Evidence of a temporary created by the host on the destination volume.
#[derive(Debug, Clone)]
pub struct VolumePreparedImport {
    pub directory: ImportPreparationDestination,
    pub file_identity: FileIdentity,
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
    /// Destination-volume temporary evidence, when staging and destination differ.
    pub volume_prepared: Option<VolumePreparedImport>,
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
