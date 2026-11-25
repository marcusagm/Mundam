use super::*;
use crate::{
    core::imports::{
        catalog::{CatalogPageRequest, ImportCatalog},
        destinations::ImportDestinations,
    },
    infra::database::{
        import_catalog::SqliteImportCatalog, import_destinations::SqliteImportDestinations,
        manager::DbManager,
    },
};

#[tokio::test]
async fn pairing_requires_consent_and_issues_once_with_catalog_grants() -> AppResult<()> {
    let directory = tempfile::tempdir()?;
    let database = DbManager::new(&directory.path().join("catalog.sqlite")).await?;
    let pool = database.pool();
    sqlx::raw_sql(
        "INSERT INTO folders (id, name, path, parent_id) VALUES
         ('photos','Photos','/photos',NULL), ('private','Private','/private',NULL),
         ('child','Child','/photos/child','photos');",
    )
    .execute(pool)
    .await?;
    let catalog_id = SqliteImportDestinations(pool)
        .preference()
        .await?
        .catalog_id;
    let pairing = SqliteImportPairing::new(pool, [5; 32]);
    let other_identity = SqliteImportPairing::new(pool, [6; 32]);
    let invite = pairing.invite().await?;
    let scopes = [ImportScope::ReadFolders, ImportScope::ReadTags];
    assert!(other_identity
        .request(&invite.code, "Browser", &scopes)
        .await
        .is_err());
    let claim = pairing.request(&invite.code, " Browser ", &scopes).await?;
    assert!(pairing
        .request(&invite.code, "Replay", &scopes)
        .await
        .is_err());
    assert!(pairing
        .redeem(&claim.request_id, &claim.secret)
        .await
        .is_err());
    let pending = pairing.pending().await?;
    assert_eq!(pending[0].client_name, "Browser");
    assert_eq!(pending[0].requested_scopes, scopes);

    for (catalog, scope, root) in [
        ("foreign", ImportScope::ReadFolders, "photos"),
        (catalog_id.as_str(), ImportScope::ImportAssets, "photos"),
        (catalog_id.as_str(), ImportScope::ReadFolders, "child"),
    ] {
        assert!(pairing
            .approve(
                &claim.request_id,
                PairingApproval {
                    catalog_id: catalog,
                    scopes: &[scope],
                    root_ids: &[root.into()],
                }
            )
            .await
            .is_err());
    }
    pairing
        .approve(
            &claim.request_id,
            PairingApproval {
                catalog_id: &catalog_id,
                scopes: &[ImportScope::ReadFolders],
                root_ids: &["photos".into()],
            },
        )
        .await?;
    assert!(pairing
        .redeem(&claim.request_id, &new_secret())
        .await
        .is_err());
    let (first, second) = tokio::join!(
        pairing.redeem(&claim.request_id, &claim.secret),
        pairing.redeem(&claim.request_id, &claim.secret),
    );
    assert_ne!(first.is_ok(), second.is_ok());
    let credential = first.or(second)?;
    let authenticated = pairing.authenticate(&credential.bearer).await?;
    assert!(other_identity
        .authenticate(&credential.bearer)
        .await
        .is_err());
    let catalog = SqliteImportCatalog::new(pool, [8; 32]);
    let folders = catalog
        .list_folders(&authenticated.catalog, None, CatalogPageRequest::default())
        .await?;
    assert_eq!(folders.items.len(), 1);
    assert_eq!(folders.items[0].id, "photos");
    assert!(matches!(
        catalog
            .list_tags(&authenticated.catalog, None, CatalogPageRequest::default())
            .await,
        Err(AppError::Forbidden(_))
    ));
    let stored: String = sqlx::query_scalar("SELECT verifier FROM import_clients WHERE id = ?1")
        .bind(&credential.client_id)
        .fetch_one(pool)
        .await?;
    assert_ne!(stored, credential.bearer);

    let rotated = pairing.rotate(&credential.bearer).await?;
    assert_eq!(rotated.client_id, credential.client_id);
    assert!(pairing.authenticate(&credential.bearer).await.is_err());
    let renewed = pairing.authenticate(&rotated.bearer).await?;
    assert_eq!(
        renewed.catalog.grant_version,
        authenticated.catalog.grant_version + 1
    );
    pairing.revoke("foreign", &rotated.client_id).await?;
    assert!(pairing.authenticate(&rotated.bearer).await.is_ok());
    pairing.revoke(&catalog_id, &rotated.client_id).await?;
    assert!(pairing.authenticate(&rotated.bearer).await.is_err());
    assert!(pairing.rotate(&rotated.bearer).await.is_err());
    database.close().await;
    Ok(())
}

#[tokio::test]
async fn rejected_and_expired_requests_cannot_issue_credentials() -> AppResult<()> {
    let directory = tempfile::tempdir()?;
    let database = DbManager::new(&directory.path().join("catalog.sqlite")).await?;
    let pool = database.pool();
    let pairing = SqliteImportPairing::new(pool, [1; 32]);
    let catalog_id = SqliteImportDestinations(pool)
        .preference()
        .await?
        .catalog_id;
    let scopes = [ImportScope::ReadTags];
    let invite = pairing.invite().await?;
    let claim = pairing.request(&invite.code, "Rejected", &scopes).await?;
    pairing.reject(&claim.request_id).await?;
    assert!(pairing
        .approve(
            &claim.request_id,
            PairingApproval {
                catalog_id: &catalog_id,
                scopes: &scopes,
                root_ids: &[],
            }
        )
        .await
        .is_err());
    let invite = pairing.invite().await?;
    let claim = pairing.request(&invite.code, "Approved", &scopes).await?;
    pairing
        .approve(
            &claim.request_id,
            PairingApproval {
                catalog_id: &catalog_id,
                scopes: &scopes,
                root_ids: &[],
            },
        )
        .await?;
    sqlx::query("UPDATE import_pairings SET expires_at = unixepoch()")
        .execute(pool)
        .await?;
    assert!(pairing
        .redeem(&claim.request_id, &claim.secret)
        .await
        .is_err());
    assert!(pairing.pending().await?.is_empty());
    let invitation = pairing.invite().await?;
    sqlx::query("UPDATE import_pairings SET expires_at = unixepoch()")
        .execute(pool)
        .await?;
    assert!(pairing
        .request(&invitation.code, "Expired", &scopes)
        .await
        .is_err());
    database.close().await;
    Ok(())
}
