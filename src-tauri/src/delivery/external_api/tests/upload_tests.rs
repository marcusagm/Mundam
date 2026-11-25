use super::*;
use crate::core::imports::{pairing::IssuedCredential, staging::PART_BYTES};
use sha2::{Digest, Sha256};

async fn pair(
    pairing: &SqliteImportPairing<'_>,
    catalog: &str,
    scopes: &[ImportScope],
) -> AppResult<IssuedCredential> {
    let invitation = pairing.invite().await?;
    let claim = pairing
        .request(&invitation.code, "Staging test", scopes)
        .await?;
    pairing
        .approve(
            &claim.request_id,
            PairingApproval {
                catalog_id: catalog,
                scopes,
                root_ids: &["received".into()],
            },
        )
        .await?;
    pairing.redeem(&claim.request_id, &claim.secret).await
}

async fn admit(
    app: &Router,
    bearer: &str,
    key: &str,
    manifest: &Value,
) -> Result<(StatusCode, Value), Box<dyn std::error::Error>> {
    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/imports")
        .header(header::HOST, AUTHORITY)
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .header(header::CONTENT_TYPE, "application/json")
        .header("idempotency-key", key)
        .body(Body::from(manifest.to_string()))?;
    let response = app.clone().oneshot(request).await?;
    let status = response.status();
    let json = serde_json::from_slice(&body::to_bytes(response.into_body(), 64 * 1024).await?)?;
    Ok((status, json))
}

async fn part(
    app: &Router,
    bearer: &str,
    batch: &str,
    number: u32,
    bytes: &[u8],
    digest: Option<&str>,
) -> Result<StatusCode, Box<dyn std::error::Error>> {
    let request = Request::builder()
        .method(Method::PUT)
        .uri(format!("/v1/imports/{batch}/items/image/parts/{number}"))
        .header(header::HOST, AUTHORITY)
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(
            "x-part-sha256",
            digest
                .map(str::to_owned)
                .unwrap_or_else(|| format!("{:x}", Sha256::digest(bytes))),
        )
        .body(Body::from(bytes.to_vec()))?;
    Ok(app.clone().oneshot(request).await?.status())
}

#[tokio::test]
async fn stages_owned_parts_reopens_and_cancels_without_publication() -> TestResult {
    let directory = tempfile::tempdir()?;
    let destination = directory.path().join("received");
    tokio::fs::create_dir(&destination).await?;
    let database = DbManager::new(&directory.path().join("catalog.sqlite")).await?;
    let pool = database.pool();
    sqlx::query("INSERT INTO folders (id, name, path) VALUES ('received', 'Received', ?), ('private', 'Private', '/private')")
        .bind(destination.to_string_lossy().as_ref()).execute(pool).await?;
    let destinations = SqliteImportDestinations(pool);
    let catalog = destinations.preference().await?.catalog_id;
    destinations.set_default(&catalog, Some("received")).await?;
    let pairing = SqliteImportPairing::new(pool, SERVER_KEY);
    let owner = pair(
        &pairing,
        &catalog,
        &[ImportScope::ImportAssets, ImportScope::AssignTagsOnImport],
    )
    .await?;
    let other = pair(&pairing, &catalog, &[ImportScope::ImportAssets]).await?;
    let staging_path = directory.path().join("staging.sqlite");
    let reopen = || async {
        Ok::<_, Box<dyn std::error::Error>>(router(
            pool.clone(),
            AUTHORITY.parse()?,
            SERVER_KEY,
            [20; 32],
            Some(SqliteImportStaging::open(&staging_path).await?),
        )?)
    };
    let app = reopen().await?;
    let manifest = json!({"items": [{"itemId": "image", "fileName": "image.png", "byteLength": PART_BYTES + 3}]});
    let mut tagged = manifest.clone();
    tagged["tagIds"] = json!(["missing"]);
    assert_eq!(admit(&app, &other.bearer, "tags", &tagged).await?.0, 403);
    assert_eq!(
        admit(&app, &owner.bearer, "tags", &tagged).await?.1["error"]["code"],
        "TAG_UNAVAILABLE"
    );
    let (status, reservation) = admit(&app, &owner.bearer, "image", &manifest).await?;
    assert_eq!(status, 201);
    let batch = reservation["batchId"].as_str().ok_or("Missing batch")?;
    let receipt = reservation["receiptPath"]
        .as_str()
        .ok_or("Missing receipt")?;
    destinations.set_default(&catalog, Some("private")).await?;
    assert_eq!(
        admit(&app, &owner.bearer, "image", &manifest).await?.1,
        reservation
    );
    assert_eq!(
        send(&app, Method::GET, receipt, Some(&other.bearer), None)
            .await?
            .0,
        404
    );
    assert_eq!(
        part(&app, &owner.bearer, batch, 1, b"end", Some("bad")).await?,
        422
    );
    assert_eq!(
        part(&app, &owner.bearer, batch, 1, b"end", None).await?,
        204
    );
    assert_eq!(
        part(&app, &owner.bearer, batch, 1, b"end", None).await?,
        204
    );
    assert_eq!(
        part(&app, &owner.bearer, batch, 1, b"new", None).await?,
        409
    );
    drop(app);
    let app = reopen().await?;
    let (_, saved) = send(&app, Method::GET, receipt, Some(&owner.bearer), None).await?;
    assert_eq!(saved["confirmedParts"][0]["partNumber"], 1);
    assert_eq!(
        part(
            &app,
            &owner.bearer,
            batch,
            0,
            &vec![0; PART_BYTES + 1],
            None
        )
        .await?,
        413
    );
    assert_eq!(
        part(&app, &owner.bearer, batch, 0, &vec![0; PART_BYTES], None).await?,
        204
    );
    let (_, page) = send(
        &app,
        Method::GET,
        &format!("{receipt}?limit=1"),
        Some(&owner.bearer),
        None,
    )
    .await?;
    assert_eq!(page["state"], "ready");
    assert_eq!(page["confirmedParts"][0]["partNumber"], 0);
    let cursor = page["nextCursor"].as_str().ok_or("Missing part cursor")?;
    let (_, page) = send(
        &app,
        Method::GET,
        &format!("{receipt}?cursor={cursor}"),
        Some(&owner.bearer),
        None,
    )
    .await?;
    assert_eq!(page["confirmedParts"][0]["partNumber"], 1);
    assert_eq!(
        send(
            &app,
            Method::POST,
            &format!("{receipt}/commit"),
            Some(&owner.bearer),
            None
        )
        .await?
        .0,
        501
    );
    assert!(tokio::fs::read_dir(&destination)
        .await?
        .next_entry()
        .await?
        .is_none());
    let assets: i64 = sqlx::query_scalar("SELECT count(*) FROM assets")
        .fetch_one(pool)
        .await?;
    assert_eq!(assets, 0);
    let (status, cancelled) =
        send(&app, Method::DELETE, receipt, Some(&owner.bearer), None).await?;
    assert_eq!(status, 202);
    assert_eq!(cancelled["state"], "cancelled");
    assert_eq!(cancelled["confirmedParts"], json!([]));
    assert_eq!(
        part(&app, &owner.bearer, batch, 1, b"end", None).await?,
        409
    );
    let large = json!({"folderId": "received", "items": [
        {"itemId": "first", "fileName": "first.bin", "byteLength": 16 * 1024 * 1024},
        {"itemId": "second", "fileName": "second.bin", "byteLength": 16 * 1024 * 1024}
    ]});
    assert_eq!(
        admit(&app, &owner.bearer, "large-one", &large).await?.0,
        201
    );
    let (status, large_reservation) = admit(&app, &owner.bearer, "large-two", &large).await?;
    assert_eq!(status, 201);
    let (status, rejected) = admit(&app, &owner.bearer, "large-three", &large).await?;
    assert_eq!(status, 429);
    assert_eq!(rejected["error"]["retryable"], true);
    let large_receipt = large_reservation["receiptPath"]
        .as_str()
        .ok_or("Missing large batch receipt")?;
    assert_eq!(
        send(
            &app,
            Method::DELETE,
            large_receipt,
            Some(&owner.bearer),
            None
        )
        .await?
        .0,
        202
    );
    assert_eq!(
        admit(&app, &owner.bearer, "large-three", &large).await?.0,
        201
    );
    database.close().await;
    Ok(())
}
