use std::path::Path;
use std::sync::Arc;
use tauri::http::{header, Request, Response, StatusCode};
use tauri::{AppHandle, Manager};

use crate::core::repository::AssetQueryHandler;
use crate::delivery::protocols::common::{boxed_error_response, error_response, serve_file};
use percent_encoding::percent_decode_str;

/// Setup the custom asset:// protocol for V2 Hexagonal Architecture
///
/// This protocol handler serves local physical media over HTTP, using
/// asynchronous `tokio::fs` calls to stream data chunks and support
/// HTTP 206 Partial Content queries. This prevents OOM errors when opening
/// massive files (e.g. 12GB MKV).
///
/// The handler supports the following URI formats:
/// - `asset://localhost/{asset_id}`: Serves the original file.
/// - `asset://localhost/{asset_id}?type=thumb`: Serves the thumbnail.
/// - `asset://localhost/{asset_id}?type=preview`: Serves a high-resolution preview.
///
/// # Arguments
/// * `app_handle`: The Tauri application handle.
/// * `request`: The HTTP request.
///
/// # Returns
/// A `Response<Vec<u8>>` containing the file data.
///
/// # Examples
/// ```no_run
/// use tauri::http::{Request, Response, StatusCode};
/// use tauri::{AppHandle, Manager};
/// use crate::delivery::protocols::asset::handler;
///
/// let mut request = Request::builder()
///     .uri("asset://localhost/12345")
///     .body(Vec::new())
///     .unwrap();
///
/// let response = handler(&app_handle, &request);
/// assert_eq!(response.status(), StatusCode::OK);
/// ```
pub async fn handler_async<R: tauri::Runtime>(
    application_handle: &AppHandle<R>,
    request: &Request<Vec<u8>>,
) -> Response<Vec<u8>> {
    let admission = match application_handle
        .try_state::<Arc<crate::delivery::protocols::admission::ProtocolAdmission>>()
    {
        Some(admission) => admission,
        None => {
            tracing::error!("Asset protocol admission state not found");
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                b"Protocol admission unavailable".to_vec(),
            );
        }
    };
    let _request_guard = match admission.enter() {
        Some(guard) => guard,
        None => {
            tracing::debug!("Asset protocol request rejected during shutdown");
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                b"Application is shutting down".to_vec(),
            );
        }
    };
    let uri = request.uri().to_string();

    let (asset_id, requested_type) = parse_asset_uri(&uri);
    let is_thumb = requested_type == Some("thumb".to_string())
        || uri.starts_with("thumb://")
        || uri.starts_with("http://thumb.localhost")
        || uri.starts_with("https://thumb.localhost");
    let is_preview = requested_type == Some("preview".to_string());
    let is_glb = requested_type == Some("glb".to_string());

    // 1. Fetch the Repository (AssetQueryHandler) from the DI state
    let query_handler = match application_handle.try_state::<Arc<dyn AssetQueryHandler>>() {
        Some(handler) => handler,
        None => {
            tracing::error!("AssetQueryHandler state not found in Tauri AppHandle");
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                b"Query handler not found".to_vec(),
            );
        }
    };

    // 2. Fetch the Asset asynchronously
    let asset_result = query_handler.get_by_id(&asset_id).await;

    let asset = match asset_result {
        Ok(Some(retrieved_asset)) => retrieved_asset,
        Ok(None) => {
            return error_response(
                StatusCode::NOT_FOUND,
                b"Asset not found in database".to_vec(),
            )
        }
        Err(database_error) => {
            tracing::error!(
                "Database error fetching asset {}: {:?}",
                asset_id,
                database_error
            );
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                b"Database error".to_vec(),
            );
        }
    };

    // 3. Resolve physical path based on type and trash state
    let mut physical_path = if asset.deleted_at.is_some() {
        if let Ok(directory) = application_handle.path().app_local_data_dir() {
            crate::core::trash::resolve_physical_path(&asset, &directory)
        } else {
            asset.path.clone()
        }
    } else {
        asset.path.clone()
    };

    if is_thumb {
        let app_data = match application_handle.path().app_local_data_dir() {
            Ok(directory) => directory,
            Err(_) => {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    b"Data configuration path not found".to_vec(),
                )
            }
        };
        let Some(filename) = asset.thumbnail_path.as_deref().filter(|filename| {
            Path::new(filename)
                .file_name()
                .and_then(|name| name.to_str())
                == Some(*filename)
        }) else {
            return error_response(StatusCode::NOT_FOUND, b"No committed thumbnail".to_vec());
        };
        physical_path = app_data.join("thumbnails").join(filename);
    } else if is_glb {
        // Only use thumbnail cache if original is NOT already a GLB/GLTF
        let path_string = physical_path.to_string_lossy().to_lowercase();
        if !path_string.ends_with(".glb") && !path_string.ends_with(".gltf") {
            let app_data = match application_handle.path().app_local_data_dir() {
                Ok(directory) => directory,
                Err(_) => {
                    return error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        b"Data configuration path not found".to_vec(),
                    )
                }
            };
            let committed_preview = if let Some(repository) = application_handle
                .try_state::<Arc<dyn crate::core::processing::ProcessingRepository>>()
            {
                match repository.preview_path(&asset.id).await {
                    Ok(path) => path,
                    Err(error) => {
                        tracing::warn!(%error, "Could not resolve committed model preview");
                        return error_response(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            b"Preview unavailable".to_vec(),
                        );
                    }
                }
            } else {
                None
            };
            let filename = committed_preview.unwrap_or_else(|| format!("{}.glb", asset.id));
            if Path::new(&filename)
                .file_name()
                .and_then(|name| name.to_str())
                != Some(filename.as_str())
            {
                return error_response(
                    StatusCode::NOT_FOUND,
                    b"Invalid preview cache path".to_vec(),
                );
            }
            physical_path = app_data.join("thumbnails").join(filename);
        }
    } else if is_preview {
        // High-Resolution Preview Logic with Lazy Disk Cache
        //
        // Previews are NOT pre-generated by the ThumbnailWorker.
        // Instead, they are generated on-demand here and cached to `previews/`
        // for subsequent requests.
        let app_data_directory = match application_handle.path().app_local_data_dir() {
            Ok(directory) => directory,
            Err(_) => {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    b"Data configuration path not found".to_vec(),
                )
            }
        };
        let preview_cache_directory = app_data_directory.join("previews");

        // Check for cached preview files (try common image extensions)
        let cached_extensions = ["png", "jpg", "jpeg", "webp"];
        let mut found_cached_preview = false;
        for candidate_extension in &cached_extensions {
            let cached_preview_path =
                preview_cache_directory.join(format!("{}.{}", asset.id, candidate_extension));
            if cached_preview_path.exists() {
                physical_path = cached_preview_path;
                found_cached_preview = true;
                break;
            }
        }

        // If no cached preview found, generate on-demand via FormatProvider and cache it
        if !found_cached_preview {
            let registry =
                match application_handle.try_state::<Arc<crate::core::formats::FormatRegistry>>() {
                    Some(registry_state) => registry_state,
                    None => {
                        return error_response(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            b"FormatRegistry not found".to_vec(),
                        )
                    }
                };

            if let Some(provider) = registry.inner().resolve(&physical_path, &[]) {
                if let Some(preview_capability) = provider.preview() {
                    let preview_result = preview_capability
                        .generate_preview(&physical_path, &asset.id)
                        .await;

                    if let Ok((data, mime)) = preview_result {
                        // Cache to disk for future requests
                        let cache_extension = match mime.as_str() {
                            "image/png" => "png",
                            "image/jpeg" | "image/jpg" => "jpg",
                            "image/webp" => "webp",
                            _ => "bin",
                        };
                        let _ = std::fs::create_dir_all(&preview_cache_directory);
                        let cached_output_path = preview_cache_directory
                            .join(format!("{}.{}", asset.id, cache_extension));
                        if let Err(cache_write_error) = std::fs::write(&cached_output_path, &data) {
                            tracing::warn!(
                                "Failed to cache preview for {}: {}",
                                asset.id,
                                cache_write_error
                            );
                        }

                        if matches!(query_handler.get_by_id(&asset.id).await, Ok(None)) {
                            let _ = tokio::fs::remove_file(&cached_output_path).await;
                            return error_response(
                                StatusCode::NOT_FOUND,
                                b"Asset removed during preview generation".to_vec(),
                            );
                        }

                        return Response::builder()
                            .status(StatusCode::OK)
                            .header(header::CONTENT_TYPE, mime)
                            .header(header::CONTENT_LENGTH, data.len().to_string())
                            .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                            .header(header::CACHE_CONTROL, "public, max-age=3600")
                            .body(data)
                            .unwrap_or_else(|_| Response::default());
                    }
                }
            }
        }
        // Fallback: if cached file was found, it will be served by serve_file_async below.
        // If generation also failed, falls through to serve the original file.
    }

    if !physical_path.exists() {
        return error_response(
            StatusCode::NOT_FOUND,
            b"File not found on local disk".to_vec(),
        );
    }

    // 4. Delegate to the async chunking I/O logic
    let range_header = request.headers().get(header::RANGE).cloned();

    let response_result = serve_file_async(&physical_path, range_header.as_ref()).await;

    match response_result {
        Ok(success_response) => success_response,
        Err(error_response_value) => *error_response_value,
    }
}

/// Synchronous wrapper around `handler_async` for callers that require a sync signature.
pub fn handler<R: tauri::Runtime>(
    application_handle: &AppHandle<R>,
    request: &Request<Vec<u8>>,
) -> Response<Vec<u8>> {
    tauri::async_runtime::block_on(handler_async(application_handle, request))
}

/// Helper to decode URI, and test if `type=thumb` is present.
///
/// # Arguments
/// * `uri`: The URI to parse.
///
/// # Returns
/// A tuple containing the asset ID and an optional string indicating the requested type.
///
/// # Examples
/// ```no_run
/// use crate::delivery::protocols::asset::parse_asset_uri;
///
/// let (asset_id, req_type) = parse_asset_uri("asset://localhost/12345?type=thumb");
/// assert_eq!(asset_id, "12345");
/// assert_eq!(req_type, Some("thumb".to_string()));
/// ```
fn parse_asset_uri(uri: &str) -> (String, Option<String>) {
    let prefixes = [
        "asset://localhost/",
        "asset://",
        "http://asset.localhost/",
        "https://asset.localhost/",
        "thumb://localhost/",
        "thumb://",
        "http://thumb.localhost/",
        "https://thumb.localhost/",
    ];

    let mut path_with_query = uri;
    for prefix in &prefixes {
        if let Some(rest) = uri.strip_prefix(prefix) {
            path_with_query = rest;
            break;
        }
    }

    let (path_part, query_part) = if let Some(pos) = path_with_query.find('?') {
        (&path_with_query[..pos], Some(&path_with_query[pos + 1..]))
    } else {
        (path_with_query, None)
    };

    let mut decoded_id = percent_decode_str(path_part)
        .decode_utf8_lossy()
        .into_owned();

    // If it's a thumbnail request, the path-part might be "uuid.webp". Strip extension to get pure UUID ID.
    if let Some(pos) = decoded_id.find('.') {
        decoded_id = decoded_id[..pos].to_string();
    }

    let req_type = query_part.and_then(|q| {
        if q.contains("type=thumb") {
            Some("thumb".to_string())
        } else if q.contains("type=preview") {
            Some("preview".to_string())
        } else if q.contains("type=glb") {
            Some("glb".to_string())
        } else {
            None
        }
    });

    (decoded_id, req_type)
}

/// Async tokio file streaming and HTTP Range chunking mechanism
///
/// # Arguments
/// * `path`: The path to the file to serve.
/// * `range`: The range header to use for partial content.
///
/// # Returns
/// A `Result<Response<Vec<u8>>, Box<Response<Vec<u8>>>>` containing the file data.
///
/// # Errors
/// Returns a boxed HTTP failure response for inaccessible files or metadata.
async fn serve_file_async(
    path: &Path,
    range: Option<&tauri::http::HeaderValue>,
) -> Result<Response<Vec<u8>>, Box<Response<Vec<u8>>>> {
    let file = match tokio::fs::File::open(path).await {
        Ok(f) => f,
        Err(e) => {
            return Err(boxed_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                e.to_string().into_bytes(),
            ))
        }
    };

    let metadata = match file.metadata().await {
        Ok(m) => m,
        Err(_) => {
            return Err(boxed_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                b"Cannot read file metadata".to_vec(),
            ))
        }
    };

    let _file_size = metadata.len();

    // Simplistic MIME detection. Use fallback octet setup if unfound.
    serve_file(path, range).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn closed_admission_returns_unavailable_without_query_handler(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let admission =
            Arc::new(crate::delivery::protocols::admission::ProtocolAdmission::default());
        let app = tauri::test::mock_builder()
            .manage(admission.clone())
            .build(crate::application_context::<tauri::test::MockRuntime>())?;
        admission.close();
        for uri in ["asset://localhost/test", "thumb://localhost/test"] {
            let request = Request::builder().uri(uri).body(Vec::new())?;
            let response = handler_async(app.handle(), &request).await;
            assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        }
        Ok(())
    }

    #[tokio::test]
    async fn database_request_drains_before_pool_close_and_late_request_is_rejected(
    ) -> Result<(), Box<dyn std::error::Error>> {
        use crate::processing::workers::test_support::ProcessingFixture;
        use std::time::Duration;

        let fixture = ProcessingFixture::new().await?;
        fixture.asset("protocol-test", None).await?;
        tokio::fs::write(fixture.data.path().join("protocol-test.png"), b"image").await?;
        let queries: Arc<dyn AssetQueryHandler> = fixture.queries.clone();
        let admission =
            Arc::new(crate::delivery::protocols::admission::ProtocolAdmission::default());
        let app = tauri::test::mock_builder()
            .manage(admission.clone())
            .manage(queries)
            .build(crate::application_context::<tauri::test::MockRuntime>())?;
        let connection = fixture.pool.acquire().await?;
        let handle = app.handle().clone();
        let task = tokio::spawn(async move {
            let request = Request::builder()
                .uri("asset://localhost/protocol-test")
                .body(Vec::new())?;
            Ok::<_, tauri::http::Error>(handler_async(&handle, &request).await)
        });
        tokio::time::timeout(Duration::from_secs(2), async {
            while admission.active_count() == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await?;
        assert_eq!(admission.close(), 1);
        assert!(
            tokio::time::timeout(Duration::from_millis(30), admission.drain())
                .await
                .is_err()
        );
        assert!(!fixture.pool.is_closed());
        let late = Request::builder()
            .uri("thumb://localhost/protocol-test")
            .body(Vec::new())?;
        assert_eq!(
            handler_async(app.handle(), &late).await.status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
        drop(connection);
        let response = tokio::time::timeout(Duration::from_secs(2), task).await???;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.body(), b"image");
        tokio::time::timeout(Duration::from_secs(2), admission.drain()).await?;
        tokio::time::timeout(Duration::from_secs(3), fixture.pool.close()).await?;
        assert!(fixture.pool.is_closed());
        assert_eq!(
            handler_async(app.handle(), &late).await.status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
        Ok(())
    }

    #[test]
    fn test_parse_asset_uri_macos() {
        let (id, req_type) = parse_asset_uri("asset://localhost/12345?type=thumb");
        assert_eq!(id, "12345");
        assert_eq!(req_type, Some("thumb".to_string()));

        let (id2, req_type2) = parse_asset_uri("asset://localhost/abc-def?type=preview");
        assert_eq!(id2, "abc-def");
        assert_eq!(req_type2, Some("preview".to_string()));

        let (id3, req_type3) = parse_asset_uri("asset://localhost/model-id?type=glb");
        assert_eq!(id3, "model-id");
        assert_eq!(req_type3, Some("glb".to_string()));

        let (id4, req_type4) = parse_asset_uri("asset://localhost/raw-asset-id");
        assert_eq!(id4, "raw-asset-id");
        assert_eq!(req_type4, None);
    }

    #[test]
    fn test_parse_asset_uri_windows() {
        let (id, req_type) = parse_asset_uri("http://asset.localhost/12345?type=thumb");
        assert_eq!(id, "12345");
        assert_eq!(req_type, Some("thumb".to_string()));

        let (id2, req_type2) = parse_asset_uri("http://asset.localhost/abc-def?type=preview");
        assert_eq!(id2, "abc-def");
        assert_eq!(req_type2, Some("preview".to_string()));

        let (id3, req_type3) = parse_asset_uri("https://asset.localhost/model-id?type=glb");
        assert_eq!(id3, "model-id");
        assert_eq!(req_type3, Some("glb".to_string()));

        let (id4, req_type4) = parse_asset_uri("http://asset.localhost/raw-asset-id");
        assert_eq!(id4, "raw-asset-id");
        assert_eq!(req_type4, None);
    }

    #[test]
    fn test_parse_asset_uri_strip_extension() {
        let (id, req_type) = parse_asset_uri("http://asset.localhost/12345.webp?type=thumb");
        assert_eq!(id, "12345");
        assert_eq!(req_type, Some("thumb".to_string()));
    }
}
