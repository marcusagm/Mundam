//! Converts public requests and responses without accepting client-supplied grants.
use super::{bearer, ApiError, ApiState};
use crate::{
    core::imports::{
        catalog::{CatalogPage, CatalogPageRequest, ImportCatalog, ImportFolder, ImportTag},
        destinations::ImportDestinations,
        pairing::{AuthenticatedImporter, ImportPairing, ImportScope, IssuedCredential},
        staging::LIMITS,
    },
    infra::database::{
        import_catalog::SqliteImportCatalog, import_destinations::SqliteImportDestinations,
    },
};
use axum::{
    extract::{
        rejection::{JsonRejection, QueryRejection},
        Query, State,
    },
    http::{HeaderMap, StatusCode},
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

pub(super) async fn capabilities(
    State(state): State<ApiState>,
    Extension(client): Extension<Arc<AuthenticatedImporter>>,
) -> Json<Value> {
    Json(json!({
        "apiVersion": "1",
        "grants": client.scopes,
        "uploadsAvailable": state.staging.is_some(),
        "publicationAvailable": state.publication.is_some(),
        "mediaTypes": if state.publication.is_some() { &["image/png"][..] } else { &[][..] },
        "limits": state.staging.as_ref().map(|_| json!({
            "maxItems": LIMITS.max_items, "maxTagsPerItem": LIMITS.max_tags_per_item,
            "maxItemBytes": LIMITS.max_item_bytes, "maxBatchBytes": LIMITS.max_batch_bytes
        })),
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct FoldersQuery {
    parent_id: Option<String>,
    cursor: Option<String>,
    limit: Option<u32>,
}

pub(super) async fn folders(
    State(state): State<ApiState>,
    Extension(client): Extension<Arc<AuthenticatedImporter>>,
    query: Result<Query<FoldersQuery>, QueryRejection>,
) -> Result<Json<CatalogPage<ImportFolder>>, ApiError> {
    let Query(query) = query.map_err(|_| ApiError::Invalid)?;
    let page = SqliteImportCatalog::new(&state.pool, state.cursor_key)
        .list_folders(
            &client.catalog,
            query.parent_id.as_deref(),
            CatalogPageRequest {
                cursor: query.cursor.as_deref(),
                limit: query.limit,
            },
        )
        .await?;
    Ok(Json(page))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TagsQuery {
    query: Option<String>,
    cursor: Option<String>,
    limit: Option<u32>,
}

pub(super) async fn tags(
    State(state): State<ApiState>,
    Extension(client): Extension<Arc<AuthenticatedImporter>>,
    query: Result<Query<TagsQuery>, QueryRejection>,
) -> Result<Json<CatalogPage<ImportTag>>, ApiError> {
    let Query(query) = query.map_err(|_| ApiError::Invalid)?;
    let page = SqliteImportCatalog::new(&state.pool, state.cursor_key)
        .list_tags(
            &client.catalog,
            query.query.as_deref(),
            CatalogPageRequest {
                cursor: query.cursor.as_deref(),
                limit: query.limit,
            },
        )
        .await?;
    Ok(Json(page))
}

pub(super) async fn import_default(
    State(state): State<ApiState>,
    Extension(client): Extension<Arc<AuthenticatedImporter>>,
) -> Result<Json<Value>, ApiError> {
    if !client.catalog.can_read_folders {
        return Err(ApiError::Forbidden);
    }
    let folder_id = SqliteImportDestinations(&state.pool)
        .resolve(&client.catalog.destinations, None)
        .await?;
    Ok(Json(json!({ "folderId": folder_id })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PairingRequest {
    code: String,
    client_name: String,
    scopes: Vec<ImportScope>,
}

pub(super) async fn request_pairing(
    State(state): State<ApiState>,
    request: Result<Json<PairingRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let Json(request) = request.map_err(json_error)?;
    let claim = state
        .pairing()
        .request(&request.code, &request.client_name, &request.scopes)
        .await
        .map_err(ApiError::credential)?;
    Ok((
        StatusCode::ACCEPTED,
        Json(json!({ "requestId": claim.request_id, "secret": claim.secret })),
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PairingClaimRequest {
    request_id: String,
    secret: String,
}

pub(super) async fn redeem_pairing(
    State(state): State<ApiState>,
    request: Result<Json<PairingClaimRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<CredentialResponse>), ApiError> {
    let Json(request) = request.map_err(json_error)?;
    let credential = state
        .pairing()
        .redeem(&request.request_id, &request.secret)
        .await
        .map_err(ApiError::credential)?;
    Ok((StatusCode::CREATED, Json(credential.into())))
}

pub(super) async fn rotate(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> Result<Json<CredentialResponse>, ApiError> {
    let credential = state
        .pairing()
        .rotate(bearer(&headers)?)
        .await
        .map_err(ApiError::credential)?;
    Ok(Json(credential.into()))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CredentialResponse {
    client_id: String,
    bearer: String,
    expires_at: i64,
}

impl From<IssuedCredential> for CredentialResponse {
    fn from(credential: IssuedCredential) -> Self {
        Self {
            client_id: credential.client_id,
            bearer: credential.bearer,
            expires_at: credential.expires_at,
        }
    }
}

pub(super) fn json_error(error: JsonRejection) -> ApiError {
    match error.status() {
        StatusCode::PAYLOAD_TOO_LARGE => ApiError::TooLarge,
        StatusCode::UNSUPPORTED_MEDIA_TYPE => ApiError::MediaType,
        _ => ApiError::Invalid,
    }
}
