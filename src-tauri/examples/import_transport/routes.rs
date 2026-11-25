//! Three owner-scoped routes exercised by the transport probe. Admission is deliberately
//! restricted to a disposable destination and an empty tag grant.
use super::staging;
use axum::{
    body::Bytes,
    extract::{rejection::JsonRejection, DefaultBodyLimit, Path, Request, State as AppState},
    http::{header, HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Extension, Json, Router,
};
use mundam_lib::core::imports::{ImportLimits, ImportManifest};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use subtle::ConstantTimeEq;
use uuid::Uuid;

#[derive(Clone)]
pub struct State {
    pub pool: SqlitePool,
    pub authority: String,
    pub owner_hash: [u8; 32],
    pub other_hash: [u8; 32],
}

#[derive(Clone, Copy)]
struct Client(&'static str);

pub enum Failure {
    Unauthorized,
    Forbidden,
    Missing,
    Expired,
    Capacity,
    TooLarge,
    Internal,
    Invalid(&'static str),
    Conflict(&'static str),
}

impl From<sqlx::Error> for Failure {
    fn from(_: sqlx::Error) -> Self {
        Self::Internal
    }
}

impl IntoResponse for Failure {
    fn into_response(self) -> Response {
        let (status, code) = match self {
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED"),
            Self::Forbidden => (StatusCode::FORBIDDEN, "FORBIDDEN"),
            Self::Missing => (StatusCode::NOT_FOUND, "NOT_FOUND"),
            Self::Expired => (StatusCode::GONE, "EXPIRED"),
            Self::Capacity => (StatusCode::TOO_MANY_REQUESTS, "PROBE_CAPACITY"),
            Self::TooLarge => (StatusCode::PAYLOAD_TOO_LARGE, "PAYLOAD_TOO_LARGE"),
            Self::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
            Self::Invalid(code) => (StatusCode::UNPROCESSABLE_ENTITY, code),
            Self::Conflict(code) => (StatusCode::CONFLICT, code),
        };
        (status, Json(json!({"requestId": Uuid::new_v4().to_string(), "error": {"code": code, "retryable": false}}))).into_response()
    }
}

pub fn router(state: State) -> Router {
    Router::new()
        .route("/v1/imports", post(admit))
        .route("/v1/imports/:batch", get(receipt))
        .route("/v1/imports/:batch/items/:item/parts/:number", put(store))
        .layer(DefaultBodyLimit::max(staging::PART_BYTES))
        .layer(middleware::from_fn_with_state(state.clone(), authenticate))
        .with_state(state)
}

async fn authenticate(
    AppState(state): AppState<State>,
    mut request: Request,
    next: Next,
) -> Result<Response, Failure> {
    let headers = request.headers();
    if headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        != Some(&state.authority)
        || headers.contains_key(header::ORIGIN)
    {
        return Err(Failure::Forbidden);
    }
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or(Failure::Unauthorized)?;
    let digest = Sha256::digest(token.as_bytes());
    let client = if bool::from(state.owner_hash.ct_eq(digest.as_slice())) {
        Client("owner")
    } else if bool::from(state.other_hash.ct_eq(digest.as_slice())) {
        Client("other")
    } else {
        return Err(Failure::Unauthorized);
    };
    request.extensions_mut().insert(client);
    Ok(next.run(request).await)
}

async fn admit(
    AppState(state): AppState<State>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    body: Result<Json<ImportManifest>, JsonRejection>,
) -> Result<(StatusCode, Json<Value>), Failure> {
    let key = headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .filter(|key| !key.trim().is_empty() && key.len() <= 128)
        .ok_or(Failure::Invalid("INVALID_KEY"))?;
    let Json(manifest) = body.map_err(|err| {
        if err.status() == StatusCode::PAYLOAD_TOO_LARGE {
            Failure::TooLarge
        } else {
            Failure::Invalid("INVALID_MANIFEST")
        }
    })?;
    let manifest = manifest
        .normalize(&ImportLimits {
            max_items: 4,
            max_tags_per_item: 8,
            max_item_bytes: 2 << 20,
            max_batch_bytes: 4 << 20,
        })
        .map_err(|_| Failure::Invalid("INVALID_MANIFEST"))?;
    if manifest.folder_id.is_none() {
        return Err(Failure::Conflict("DESTINATION_REQUIRED"));
    }
    if manifest
        .folder_id
        .as_deref()
        .is_some_and(|folder| folder != "probe-folder")
        || manifest.items.iter().any(|item| !item.tag_ids.is_empty())
    {
        return Err(Failure::Forbidden);
    }
    Ok((
        StatusCode::CREATED,
        Json(staging::admit(&state.pool, client.0, key, manifest).await?),
    ))
}

async fn receipt(
    AppState(state): AppState<State>,
    Extension(client): Extension<Client>,
    Path(batch): Path<String>,
) -> Result<Json<Value>, Failure> {
    Ok(Json(staging::receipt(&state.pool, client.0, &batch).await?))
}

async fn store(
    AppState(state): AppState<State>,
    Extension(client): Extension<Client>,
    Path((batch, item, number)): Path<(String, String, u32)>,
    headers: HeaderMap,
    body: Result<Bytes, axum::extract::rejection::BytesRejection>,
) -> Result<StatusCode, Failure> {
    if headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        != Some("application/octet-stream")
    {
        return Err(Failure::Invalid("EXPECTED_BINARY_PART"));
    }
    let bytes = body.map_err(|err| {
        if err.status() == StatusCode::PAYLOAD_TOO_LARGE {
            Failure::TooLarge
        } else {
            Failure::Invalid("INCOMPLETE_BODY")
        }
    })?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    if headers
        .get("x-part-sha256")
        .and_then(|value| value.to_str().ok())
        != Some(&digest)
    {
        return Err(Failure::Invalid("INVALID_DIGEST"));
    }
    staging::store(
        &state.pool,
        client.0,
        &batch,
        &item,
        number,
        &digest,
        &bytes,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
