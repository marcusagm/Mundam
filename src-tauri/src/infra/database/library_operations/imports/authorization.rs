//! Verifies live consent using the same SQLite writer that publishes the asset.
use crate::core::{
    imports::{
        destinations::DestinationGrant,
        pairing::ImportScope,
        publication::{PublicationAuthority, PublishPreparedImport},
    },
    AppError, AppResult,
};
use sqlx::SqliteConnection;

pub(super) async fn revalidate(
    connection: &mut SqliteConnection,
    command: &PublishPreparedImport,
) -> AppResult<()> {
    let PublicationAuthority::Paired(consent) = &command.authority else {
        return Ok(());
    };
    let required = ImportScope::ImportAssets.mask()
        | if command.item.tag_ids.is_empty() {
            0
        } else {
            ImportScope::AssignTagsOnImport.mask()
        };
    let version = i64::try_from(consent.grant_version).map_err(|_| denied())?;
    let roots: String = sqlx::query_scalar(
        "SELECT clients.roots FROM import_clients AS clients
         JOIN import_preferences AS preferences ON preferences.singleton = 1
         WHERE preferences.catalog_id = ? AND clients.id = ? AND clients.server_key = ?
           AND clients.grant_version = ? AND clients.verifier IS NOT NULL
           AND clients.expires_at > unixepoch() AND (clients.scopes & ?) = ?",
    )
    .bind(&consent.catalog_id)
    .bind(&consent.client_id)
    .bind(&consent.server_key)
    .bind(version)
    .bind(required)
    .bind(required)
    .fetch_optional(&mut *connection)
    .await?
    .ok_or_else(denied)?;
    let grant = DestinationGrant {
        catalog_id: consent.catalog_id.clone(),
        root_ids: serde_json::from_str(&roots)
            .map_err(|error| AppError::Internal(error.to_string()))?,
    };
    super::super::super::import_destinations::resolve_scoped(
        connection,
        &grant,
        Some(&command.folder_id),
    )
    .await?
    .ok_or_else(denied)?;
    Ok(())
}

fn denied() -> AppError {
    AppError::Forbidden("Import publication is outside the current consent".into())
}
