//! Persists the exact destination-volume file before writing staged bytes to it.
use super::SqliteImportStaging;
use crate::{
    core::{
        imports::{
            publication::{ImportPreparationDestination, VolumePreparedImport},
            staging::UploadError,
        },
        AppError,
    },
    infra::filesystem::prepared_imports,
};
use sqlx::{Sqlite, SqliteConnection, Transaction};
use std::path::Path;
use tokio::fs::File;
use uuid::Uuid;

pub(super) async fn prepare_volume<'staging>(
    staging: &'staging SqliteImportStaging,
    writer: Transaction<'staging, Sqlite>,
    attempt_id: Uuid,
    destination: ImportPreparationDestination,
) -> Result<
    (
        Transaction<'staging, Sqlite>,
        File,
        VolumePreparedImport,
        prepared_imports::VolumeDirectoryLease,
    ),
    UploadError,
> {
    #[cfg(windows)]
    {
        let mut writer = writer;
        let attempt = attempt_id.to_string();
        sqlx::query(
            "INSERT INTO import_volume_temporaries (attempt_id, directory_json) VALUES (?, ?)",
        )
        .bind(&attempt)
        .bind(json(&destination)?)
        .execute(&mut *writer)
        .await?;
        writer.commit().await?;

        let mut writer = begin_live(staging, &attempt).await?;
        let (file, file_identity, directory_lease) =
            prepared_imports::create_on_volume(&destination, attempt_id).await?;
        sqlx::query("UPDATE import_volume_temporaries SET identity_json = ? WHERE attempt_id = ?")
            .bind(json(&file_identity)?)
            .bind(&attempt)
            .execute(&mut *writer)
            .await?;
        writer.commit().await?;

        let writer = begin_live(staging, &attempt).await?;
        Ok((
            writer,
            file,
            VolumePreparedImport {
                directory: destination,
                file_identity,
            },
            directory_lease,
        ))
    }
    #[cfg(not(windows))]
    {
        let _ = (staging, writer, attempt_id, destination);
        Err(AppError::ValidationFailed(
            "Cross-volume import is unsupported on this platform".into(),
        )
        .into())
    }
}

#[cfg(windows)]
async fn begin_live<'staging>(
    staging: &'staging SqliteImportStaging,
    attempt: &str,
) -> Result<Transaction<'staging, Sqlite>, UploadError> {
    let mut writer = staging.0.begin_with("BEGIN IMMEDIATE").await?;
    let live: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM import_volume_temporaries AS temporary
         JOIN import_publication_claims AS claims ON temporary.attempt_id = claims.attempt_id
         JOIN import_batches AS batches ON batches.id = claims.batch_id
         WHERE temporary.attempt_id = ? AND batches.cancelled = 0 AND batches.expires > unixepoch())",
    ).bind(attempt).fetch_one(&mut *writer).await?;
    if !live {
        return Err(UploadError::Missing);
    }
    Ok(writer)
}

pub(super) async fn cleanup(
    writer: &mut SqliteConnection,
    app_data: &Path,
    attempt_id: Uuid,
) -> Result<(), UploadError> {
    let location: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT directory_json, identity_json FROM import_volume_temporaries WHERE attempt_id = ?",
    )
    .bind(attempt_id.to_string())
    .fetch_optional(writer)
    .await?;
    if let Some((directory, identity)) = location {
        #[cfg(windows)]
        {
            let directory =
                serde_json::from_str(&directory).map_err(|_| UploadError::StagingCorrupt)?;
            let identity = identity
                .as_deref()
                .map(serde_json::from_str)
                .transpose()
                .map_err(|_| UploadError::StagingCorrupt)?;
            prepared_imports::remove_on_volume(&directory, attempt_id, identity.as_ref()).await?;
        }
        #[cfg(not(windows))]
        {
            let _ = (directory, identity);
            return Err(AppError::ValidationFailed(
                "Destination-volume recovery requires Windows".into(),
            )
            .into());
        }
    } else {
        prepared_imports::remove(app_data, attempt_id).await?;
    }
    Ok(())
}

#[cfg(windows)]
fn json(value: &impl serde::Serialize) -> Result<String, UploadError> {
    serde_json::to_string(value).map_err(|error| AppError::Internal(error.to_string()).into())
}
