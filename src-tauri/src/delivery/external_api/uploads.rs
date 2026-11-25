//! Public staging routes. Publication remains a separate Ledger operation.
use super::{bearer, handlers::json_error, read_single_header, ApiError, ApiState};
use crate::{
    core::imports::{
        destinations::ImportDestinations,
        pairing::{AuthenticatedImporter, ImportPairing, ImportScope},
        staging::{UploadAdmission, UploadError, UploadOwner, UploadReceipt, LIMITS},
        ImportManifest,
    },
    infra::database::{
        import_catalog::contains_import_tags, import_destinations::SqliteImportDestinations,
        import_staging::SqliteImportStaging,
    },
};
use axum::{
    body::Bytes,
    extract::{
        rejection::{BytesRejection, JsonRejection, PathRejection, QueryRejection},
        Path, Query, State,
    },
    http::{header, HeaderMap, HeaderName, StatusCode},
    Extension, Json,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;

fn owner(client: &AuthenticatedImporter) -> Result<UploadOwner<'_>, ApiError> {
    if !client.scopes.contains(&ImportScope::ImportAssets) {
        return Err(ApiError::Forbidden);
    }
    Ok(UploadOwner {
        catalog_id: &client.catalog.destinations.catalog_id,
        client_id: &client.catalog.client_id,
    })
}

fn staging(state: &ApiState) -> Result<&SqliteImportStaging, ApiError> {
    state.staging.as_ref().ok_or(ApiError::Missing)
}

pub(super) async fn admit(
    State(state): State<ApiState>,
    Extension(client): Extension<Arc<AuthenticatedImporter>>,
    headers: HeaderMap,
    body: Result<Json<ImportManifest>, JsonRejection>,
) -> Result<(StatusCode, Json<UploadAdmission>), ApiError> {
    let owner = owner(&client)?;
    let staging = staging(&state)?;
    let key = read_single_header(&headers, HeaderName::from_static("idempotency-key"))
        .filter(|key| !key.trim().is_empty() && key.len() <= 128)
        .ok_or(ApiError::Invalid)?;
    let Json(manifest) = body.map_err(json_error)?;
    let manifest = manifest.normalize(&LIMITS)?;
    let tag_ids: Vec<_> = manifest
        .items
        .iter()
        .flat_map(|item| &item.tag_ids)
        .collect();
    if !tag_ids.is_empty() && !client.scopes.contains(&ImportScope::AssignTagsOnImport) {
        return Err(ApiError::Forbidden);
    }
    let canonical = serde_json::to_string(&manifest).map_err(|_| ApiError::Internal)?;
    if let Some(existing) = staging.replay(&owner, key, &canonical).await? {
        return Ok((StatusCode::CREATED, Json(existing)));
    }
    let folder_id = SqliteImportDestinations(&state.pool)
        .resolve(&client.catalog.destinations, manifest.folder_id.as_deref())
        .await?
        .ok_or(ApiError::Conflict("DESTINATION_REQUIRED"))?;
    if !tag_ids.is_empty() && !contains_import_tags(&state.pool, &tag_ids).await? {
        return Err(ApiError::InvalidUpload("TAG_UNAVAILABLE"));
    }
    let reservation = staging
        .reserve(&owner, key, &canonical, &manifest, &folder_id)
        .await?;
    Ok((StatusCode::CREATED, Json(reservation)))
}

pub(super) async fn upload_part(
    State(state): State<ApiState>,
    path: Result<Path<(String, String, u32)>, PathRejection>,
    headers: HeaderMap,
    body: Result<Bytes, BytesRejection>,
) -> Result<StatusCode, ApiError> {
    let Path((batch, item, number)) = path.map_err(|_| ApiError::Invalid)?;
    if read_single_header(&headers, header::CONTENT_TYPE) != Some("application/octet-stream") {
        return Err(ApiError::MediaType);
    }
    let bytes = body.map_err(|error| {
        if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
            ApiError::TooLarge
        } else {
            ApiError::InvalidUpload("INCOMPLETE_BODY")
        }
    })?;
    // Recheck the bearer after reading the body to catch revocations during transfer.
    let client = state
        .pairing()
        .authenticate(bearer(&headers)?)
        .await
        .map_err(ApiError::credential)?;
    let owner = owner(&client)?;
    let staging = staging(&state)?;
    let folder_id = staging.destination(&owner, &batch).await?;
    SqliteImportDestinations(&state.pool)
        .resolve(&client.catalog.destinations, Some(&folder_id))
        .await?
        .ok_or(ApiError::Conflict("DESTINATION_UNAVAILABLE"))?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    if read_single_header(&headers, HeaderName::from_static("x-part-sha256")) != Some(&digest) {
        return Err(ApiError::InvalidUpload("INVALID_DIGEST"));
    }
    staging
        .store(&owner, &batch, &item, number, &digest, &bytes)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReceiptQuery {
    cursor: Option<String>,
    limit: Option<u32>,
}

pub(super) async fn receipt(
    State(state): State<ApiState>,
    Extension(client): Extension<Arc<AuthenticatedImporter>>,
    path: Result<Path<String>, PathRejection>,
    query: Result<Query<ReceiptQuery>, QueryRejection>,
) -> Result<Json<UploadReceipt>, ApiError> {
    let Path(batch) = path.map_err(|_| ApiError::Invalid)?;
    let Query(query) = query.map_err(|_| ApiError::Invalid)?;
    Ok(Json(
        staging(&state)?
            .receipt(
                &owner(&client)?,
                &batch,
                query.cursor.as_deref(),
                query.limit,
            )
            .await?,
    ))
}

pub(super) async fn cancel(
    State(state): State<ApiState>,
    Extension(client): Extension<Arc<AuthenticatedImporter>>,
    path: Result<Path<String>, PathRejection>,
) -> Result<(StatusCode, Json<UploadReceipt>), ApiError> {
    let Path(batch) = path.map_err(|_| ApiError::Invalid)?;
    Ok((
        StatusCode::ACCEPTED,
        Json(staging(&state)?.cancel(&owner(&client)?, &batch).await?),
    ))
}

pub(super) async fn publication_unavailable(
    State(state): State<ApiState>,
    Extension(client): Extension<Arc<AuthenticatedImporter>>,
    Path(batch): Path<String>,
) -> Result<(), ApiError> {
    staging(&state)?
        .destination(&owner(&client)?, &batch)
        .await?;
    Err(ApiError::PublicationUnavailable)
}

impl From<UploadError> for ApiError {
    fn from(error: UploadError) -> Self {
        match error {
            UploadError::Missing => Self::Missing,
            UploadError::Expired => Self::ImportExpired,
            UploadError::Cancelled => Self::Conflict("IMPORT_CANCELLED"),
            UploadError::IdempotencyConflict => Self::Conflict("IDEMPOTENCY_CONFLICT"),
            UploadError::PartConflict => Self::Conflict("PART_CONFLICT"),
            UploadError::InvalidPart => Self::InvalidUpload("INVALID_PART_SIZE"),
            UploadError::Quota => Self::ImportQuota,
            UploadError::DiskFull => Self::DiskFull,
            UploadError::Other(error) => error.into(),
        }
    }
}
