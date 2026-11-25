//! Local import HTTP adapter. The listener must serve it over HTTPS on IPv4 loopback.
pub(crate) mod desktop;
#[cfg(windows)]
mod discovery;
mod handlers;
pub mod listener;
mod publication;
mod uploads;

use crate::{
    core::{imports::pairing::ImportPairing, AppError, AppResult},
    infra::database::{import_pairing::SqliteImportPairing, import_staging::SqliteImportStaging},
};
use axum::{
    extract::{DefaultBodyLimit, Request, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Json, Router,
};
use serde_json::json;
use sqlx::SqlitePool;
use std::{
    net::SocketAddrV4,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use uuid::Uuid;

#[derive(Clone)]
struct ApiState {
    pool: SqlitePool,
    authority: String,
    server_key: [u8; 32],
    cursor_key: [u8; 32],
    pairing_window: Arc<Mutex<PairingWindow>>,
    staging: Option<SqliteImportStaging>,
    publication: Option<publication::PublicationQueue>,
}

impl ApiState {
    fn pairing(&self) -> SqliteImportPairing<'_> {
        SqliteImportPairing::new(&self.pool, self.server_key)
    }
}

struct PairingWindow {
    started_at: Instant,
    attempts: u8,
}

impl PairingWindow {
    fn admit(&mut self) -> Result<(), ApiError> {
        const WINDOW: Duration = Duration::from_secs(60);
        const MAX_ATTEMPTS: u8 = 32;
        let now = Instant::now();
        let elapsed = now.duration_since(self.started_at);
        if elapsed >= WINDOW {
            self.started_at = now;
            self.attempts = 0;
        }
        if self.attempts >= MAX_ATTEMPTS {
            return Err(ApiError::RateLimited((WINDOW - elapsed).as_secs() + 1));
        }
        self.attempts += 1;
        Ok(())
    }
}

/// Builds routes for a bound HTTPS listener and the matching public-key digest.
/// The caller supplies a private cursor key for the lifetime of the router.
///
/// # Errors
/// Rejects authorities other than an assigned port on 127.0.0.1.
pub fn router(
    pool: SqlitePool,
    address: SocketAddrV4,
    server_key: [u8; 32],
    cursor_key: [u8; 32],
    staging: Option<SqliteImportStaging>,
) -> AppResult<Router> {
    router_with_publication(pool, address, server_key, cursor_key, staging, None)
}

fn router_with_publication(
    pool: SqlitePool,
    address: SocketAddrV4,
    server_key: [u8; 32],
    cursor_key: [u8; 32],
    staging: Option<SqliteImportStaging>,
    publication: Option<publication::PublicationQueue>,
) -> AppResult<Router> {
    if *address.ip() != std::net::Ipv4Addr::LOCALHOST || address.port() == 0 {
        return Err(AppError::ValidationFailed(
            "Import API requires a bound loopback address".into(),
        ));
    }
    let state = ApiState {
        pool,
        authority: address.to_string(),
        server_key,
        cursor_key,
        staging,
        publication,
        pairing_window: Arc::new(Mutex::new(PairingWindow {
            started_at: Instant::now(),
            attempts: 0,
        })),
    };
    let paired = Router::new()
        .route("/v1/capabilities", get(handlers::capabilities))
        .route("/v1/folders", get(handlers::folders))
        .route("/v1/tags", get(handlers::tags))
        .route("/v1/import-default", get(handlers::import_default))
        .route("/v1/imports", post(uploads::admit))
        .route(
            "/v1/imports/:batch",
            get(uploads::receipt).delete(uploads::cancel),
        )
        .route("/v1/imports/:batch/commit", post(uploads::commit))
        .route(
            "/v1/imports/:batch/items/:item/parts/:number",
            put(uploads::upload_part).layer(DefaultBodyLimit::max(
                crate::core::imports::staging::PART_BYTES,
            )),
        )
        .route_layer(middleware::from_fn_with_state(state.clone(), authenticate));
    let pairing = Router::new()
        .route("/v1/pairing/requests", post(handlers::request_pairing))
        .route("/v1/pairing/claims", post(handlers::redeem_pairing))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            limit_pairing_attempts,
        ));
    Ok(paired
        .merge(pairing)
        .route("/v1/credentials/rotate", post(handlers::rotate))
        .fallback(|| async { ApiError::Missing })
        .method_not_allowed_fallback(|| async { ApiError::Method })
        .layer(DefaultBodyLimit::max(8 * 1024))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            enforce_local_request,
        ))
        .with_state(state))
}

async fn enforce_local_request(
    State(state): State<ApiState>,
    request: Request,
    next: Next,
) -> Response {
    let valid = read_single_header(request.headers(), header::HOST) == Some(&state.authority)
        && request.uri().authority().is_none()
        && !request.headers().contains_key(header::ORIGIN);
    let mut response = if valid {
        next.run(request).await
    } else {
        ApiError::Forbidden.into_response()
    };
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn read_single_header(headers: &HeaderMap, key: header::HeaderName) -> Option<&str> {
    let values = headers.get_all(key);
    let mut values = values.iter();
    let value = values.next()?.to_str().ok()?;
    values.next().is_none().then_some(value)
}

fn bearer(headers: &HeaderMap) -> Result<&str, ApiError> {
    let authorization =
        read_single_header(headers, header::AUTHORIZATION).ok_or(ApiError::Unauthorized)?;
    let (scheme, bearer) = authorization
        .split_once(' ')
        .ok_or(ApiError::Unauthorized)?;
    if !scheme.eq_ignore_ascii_case("bearer") {
        return Err(ApiError::Unauthorized);
    }
    Ok(bearer)
}

async fn authenticate(
    State(state): State<ApiState>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let importer = state
        .pairing()
        .authenticate(bearer(request.headers())?)
        .await
        .map_err(ApiError::credential)?;
    request.extensions_mut().insert(Arc::new(importer));
    Ok(next.run(request).await)
}

async fn limit_pairing_attempts(
    State(state): State<ApiState>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    state
        .pairing_window
        .lock()
        .map_err(|_| ApiError::Internal)?
        .admit()?;
    Ok(next.run(request).await)
}

enum ApiError {
    Unauthorized,
    Forbidden,
    Missing,
    Method,
    CursorExpired,
    Invalid,
    InvalidUpload(&'static str),
    TooLarge,
    MediaType,
    RateLimited(u64),
    Conflict(&'static str),
    ImportExpired,
    ImportQuota,
    DiskFull,
    PublicationUnavailable,
    PublicationBusy,
    PublicationClosing,
    Internal,
}

impl ApiError {
    fn credential(error: AppError) -> Self {
        match error {
            AppError::Forbidden(_) => Self::Unauthorized,
            other => other.into(),
        }
    }
}

impl From<AppError> for ApiError {
    fn from(error: AppError) -> Self {
        match error {
            AppError::Forbidden(_) => Self::Forbidden,
            AppError::NotFound(_) => Self::Missing,
            AppError::CursorExpired => Self::CursorExpired,
            AppError::ValidationFailed(_) => Self::Invalid,
            _ => Self::Internal,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = match self {
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED"),
            Self::Forbidden => (StatusCode::FORBIDDEN, "FORBIDDEN"),
            Self::Missing => (StatusCode::NOT_FOUND, "NOT_FOUND"),
            Self::Method => (StatusCode::METHOD_NOT_ALLOWED, "METHOD_NOT_ALLOWED"),
            Self::CursorExpired => (StatusCode::CONFLICT, "CURSOR_EXPIRED"),
            Self::Invalid => (StatusCode::UNPROCESSABLE_ENTITY, "INVALID_REQUEST"),
            Self::InvalidUpload(code) => (StatusCode::UNPROCESSABLE_ENTITY, code),
            Self::TooLarge => (StatusCode::PAYLOAD_TOO_LARGE, "PAYLOAD_TOO_LARGE"),
            Self::MediaType => (StatusCode::UNSUPPORTED_MEDIA_TYPE, "UNSUPPORTED_MEDIA_TYPE"),
            Self::RateLimited(_) => (StatusCode::TOO_MANY_REQUESTS, "PAIRING_RATE_LIMITED"),
            Self::Conflict(code) => (StatusCode::CONFLICT, code),
            Self::ImportExpired => (StatusCode::GONE, "IMPORT_EXPIRED"),
            Self::ImportQuota => (StatusCode::TOO_MANY_REQUESTS, "STAGING_QUOTA_EXCEEDED"),
            Self::DiskFull => (StatusCode::INSUFFICIENT_STORAGE, "STAGING_DISK_FULL"),
            Self::PublicationUnavailable => {
                (StatusCode::NOT_IMPLEMENTED, "PUBLICATION_UNAVAILABLE")
            }
            Self::PublicationBusy => (StatusCode::SERVICE_UNAVAILABLE, "PUBLICATION_BUSY"),
            Self::PublicationClosing => (StatusCode::SERVICE_UNAVAILABLE, "PUBLICATION_CLOSING"),
            Self::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
        };
        let mut response = (
            status,
            Json(json!({
                "requestId": Uuid::new_v4().to_string(),
                "error": {"code": code, "retryable": matches!(self, Self::RateLimited(_) | Self::ImportQuota | Self::PublicationBusy | Self::PublicationClosing)}
            })),
        )
            .into_response();
        if let Self::RateLimited(seconds) = self {
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, HeaderValue::from(seconds));
        }
        if matches!(self, Self::PublicationBusy | Self::PublicationClosing) {
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, HeaderValue::from_static("1"));
        }
        if status == StatusCode::UNAUTHORIZED {
            response
                .headers_mut()
                .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
        }
        response
    }
}

#[cfg(test)]
mod tests;
