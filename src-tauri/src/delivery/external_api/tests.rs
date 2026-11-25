use super::*;
use crate::{
    core::imports::{
        destinations::ImportDestinations,
        pairing::{ImportScope, PairingApproval},
    },
    infra::database::{import_destinations::SqliteImportDestinations, manager::DbManager},
};
use axum::{
    body::{self, Body},
    http::Method,
};
use serde_json::Value;
use tower::ServiceExt;

const AUTHORITY: &str = "127.0.0.1:49152";
const SERVER_KEY: [u8; 32] = [13; 32];
type TestResult = Result<(), Box<dyn std::error::Error>>;

async fn send(
    app: &Router,
    method: Method,
    route: &str,
    bearer: Option<&str>,
    payload: Option<Value>,
) -> Result<(StatusCode, Value), Box<dyn std::error::Error>> {
    let mut request = Request::builder()
        .method(method)
        .uri(route)
        .header(header::HOST, AUTHORITY);
    if let Some(bearer) = bearer {
        request = request.header(header::AUTHORIZATION, format!("Bearer {bearer}"));
    }
    let body = if let Some(payload) = payload {
        request = request.header(header::CONTENT_TYPE, "application/json");
        Body::from(payload.to_string())
    } else {
        Body::empty()
    };
    let response = app.clone().oneshot(request.body(body)?).await?;
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let status = response.status();
    let json = serde_json::from_slice(&body::to_bytes(response.into_body(), 128 * 1024).await?)?;
    Ok((status, json))
}

#[tokio::test]
async fn http_import_catalog_uses_approved_grants_and_live_credentials() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("photos");
    tokio::fs::create_dir(&root).await?;
    let database = DbManager::new(&directory.path().join("catalog.sqlite")).await?;
    let pool = database.pool();
    sqlx::query(
        "INSERT INTO folders (id, name, path) VALUES ('photos','Photos',?), ('private','Private','/private')",
    ).bind(root.to_string_lossy().as_ref()).execute(pool).await?;
    sqlx::raw_sql(
        "INSERT INTO folders (id, name, path, parent_id) VALUES
         ('one','One','/one','photos'), ('two','Two','/two','photos');
         INSERT INTO tags (id, name) VALUES ('portrait','Portrait');",
    )
    .execute(pool)
    .await?;
    let destinations = SqliteImportDestinations(pool);
    let catalog_id = destinations.preference().await?.catalog_id;
    destinations
        .set_default(&catalog_id, Some("photos"))
        .await?;
    let desktop = SqliteImportPairing::new(pool, SERVER_KEY);
    let invitation = desktop.invite().await?;
    let app = router(pool.clone(), AUTHORITY.parse()?, SERVER_KEY, [20; 32])?;

    assert_eq!(
        send(&app, Method::GET, "/v1/folders", None, None).await?.0,
        401
    );
    for (host, origin) in [
        (AUTHORITY, Some("https://example.com")),
        ("localhost:49152", None),
    ] {
        let mut request = Request::builder()
            .uri("/v1/folders")
            .header(header::HOST, host);
        if let Some(origin) = origin {
            request = request.header(header::ORIGIN, origin);
        }
        let response = app.clone().oneshot(request.body(Body::empty())?).await?;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    }
    let (status, claim) = send(&app, Method::POST, "/v1/pairing/requests", None, Some(json!({
        "code": invitation.code, "clientName": "Browser", "scopes": ["folders.read", "tags.read"]
    }))).await?;
    assert_eq!(status, 202);
    let id = claim["requestId"]
        .as_str()
        .ok_or("Missing claim ID")?
        .to_owned();
    assert_eq!(
        send(
            &app,
            Method::POST,
            "/v1/pairing/claims",
            None,
            Some(claim.clone())
        )
        .await?
        .0,
        401
    );
    assert_eq!(
        send(&app, Method::POST, "/v1/pairing/approve", None, None)
            .await?
            .0,
        404
    );
    desktop
        .approve(
            &id,
            PairingApproval {
                catalog_id: &catalog_id,
                scopes: &[ImportScope::ReadFolders],
                root_ids: &["photos".into()],
            },
        )
        .await?;
    let (status, issued) = send(
        &app,
        Method::POST,
        "/v1/pairing/claims",
        None,
        Some(claim.clone()),
    )
    .await?;
    assert_eq!(status, 201);
    assert_eq!(
        send(&app, Method::POST, "/v1/pairing/claims", None, Some(claim))
            .await?
            .0,
        401
    );
    let bearer = issued["bearer"].as_str().ok_or("Missing bearer")?;
    let (status, capabilities) =
        send(&app, Method::GET, "/v1/capabilities", Some(bearer), None).await?;
    assert_eq!(status, 200);
    assert_eq!(capabilities["grants"], json!(["folders.read"]));
    assert_eq!(capabilities["uploadsAvailable"], false);
    assert_eq!(
        send(&app, Method::GET, "/v1/tags", Some(bearer), None)
            .await?
            .0,
        403
    );
    assert_eq!(
        send(
            &app,
            Method::GET,
            "/v1/folders?parentId=private",
            Some(bearer),
            None
        )
        .await?
        .0,
        404
    );
    assert_eq!(
        send(
            &app,
            Method::GET,
            "/v1/folders?limit=101",
            Some(bearer),
            None
        )
        .await?
        .0,
        422
    );
    let (status, roots) = send(&app, Method::GET, "/v1/folders", Some(bearer), None).await?;
    assert_eq!(status, 200);
    assert_eq!(roots["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(roots["items"][0]["id"], "photos");
    assert!(roots["items"][0].get("path").is_none());
    assert_eq!(
        send(&app, Method::GET, "/v1/import-default", Some(bearer), None).await?,
        (StatusCode::OK, json!({"folderId": "photos"}))
    );
    let (_, page) = send(
        &app,
        Method::GET,
        "/v1/folders?parentId=photos&limit=1",
        Some(bearer),
        None,
    )
    .await?;
    let cursor = page["nextCursor"].as_str().ok_or("Missing continuation")?;
    let continuation = format!("/v1/folders?parentId=photos&limit=1&cursor={cursor}");
    let (_, page) = send(&app, Method::GET, &continuation, Some(bearer), None).await?;
    assert_eq!(page["items"][0]["id"], "two");

    let (status, rotated) = send(
        &app,
        Method::POST,
        "/v1/credentials/rotate",
        Some(bearer),
        None,
    )
    .await?;
    assert_eq!(status, 200);
    let replacement = rotated["bearer"].as_str().ok_or("Missing replacement")?;
    assert_eq!(
        send(&app, Method::GET, "/v1/folders", Some(bearer), None)
            .await?
            .0,
        401
    );
    let (status, error) = send(&app, Method::GET, &continuation, Some(replacement), None).await?;
    assert_eq!(status, 409);
    assert_eq!(error["error"]["code"], "CURSOR_EXPIRED");
    desktop.revoke(&catalog_id, &id).await?;
    assert_eq!(
        send(&app, Method::GET, "/v1/folders", Some(replacement), None)
            .await?
            .0,
        401
    );

    let invitation = desktop.invite().await?;
    let (_, claim) = send(
        &app,
        Method::POST,
        "/v1/pairing/requests",
        None,
        Some(json!({
            "code": invitation.code, "clientName": "Tag reader", "scopes": ["tags.read"]
        })),
    )
    .await?;
    desktop
        .approve(
            claim["requestId"].as_str().ok_or("Missing tag claim ID")?,
            PairingApproval {
                catalog_id: &catalog_id,
                scopes: &[ImportScope::ReadTags],
                root_ids: &[],
            },
        )
        .await?;
    let (_, issued) = send(&app, Method::POST, "/v1/pairing/claims", None, Some(claim)).await?;
    let bearer = issued["bearer"]
        .as_str()
        .ok_or("Missing tag reader bearer")?;
    let (status, tags) = send(
        &app,
        Method::GET,
        "/v1/tags?query=portrait",
        Some(bearer),
        None,
    )
    .await?;
    assert_eq!(status, 200);
    assert_eq!(tags["items"][0]["labels"], json!(["Portrait"]));
    assert_eq!(
        send(&app, Method::GET, "/v1/import-default", Some(bearer), None)
            .await?
            .0,
        403
    );
    database.close().await;
    Ok(())
}

#[tokio::test]
async fn pairing_attempts_have_a_shared_http_budget() -> TestResult {
    let directory = tempfile::tempdir()?;
    let database = DbManager::new(&directory.path().join("catalog.sqlite")).await?;
    let app = router(
        database.pool().clone(),
        AUTHORITY.parse()?,
        SERVER_KEY,
        [20; 32],
    )?;
    for _ in 0..32 {
        assert_eq!(
            send(
                &app,
                Method::POST,
                "/v1/pairing/requests",
                None,
                Some(json!({
                    "code": "invalid", "clientName": "Browser", "scopes": ["tags.read"]
                }))
            )
            .await?
            .0,
            401
        );
    }
    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/pairing/claims")
        .header(header::HOST, AUTHORITY)
        .body(Body::empty())?;
    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(response.headers().contains_key(header::RETRY_AFTER));
    database.close().await;
    Ok(())
}
