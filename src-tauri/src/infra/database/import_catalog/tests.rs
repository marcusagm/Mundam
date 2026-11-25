use super::*;
use crate::{
    core::imports::destinations::{DestinationGrant, ImportDestinations},
    infra::database::{import_destinations::SqliteImportDestinations, manager::DbManager},
};

fn request(cursor: Option<&str>) -> CatalogPageRequest<'_> {
    CatalogPageRequest {
        limit: Some(1),
        cursor,
    }
}

#[tokio::test]
async fn catalog_pages_enforce_visibility_and_expire_after_changes() -> AppResult<()> {
    let directory = tempfile::tempdir()?;
    let database = DbManager::new(&directory.path().join("catalog.sqlite")).await?;
    let pool = database.pool();
    sqlx::raw_sql(
        "INSERT INTO folders (id, name, path, parent_id) VALUES
            ('root', 'Public', '/public', NULL), ('private', 'Private', '/private', NULL),
            ('alpha', 'Same', '/public/alpha', 'root'), ('beta', 'Same', '/public/beta', 'root');
         INSERT INTO tags (id, name, parent_id, order_index) VALUES
            ('parent', 'Reference', NULL, 0), ('leaf', 'Photo', 'parent', 1),
            ('literal', 'Photograph %', NULL, 1);",
    )
    .execute(pool)
    .await?;
    let preference = SqliteImportDestinations(pool).preference().await?;
    let catalog = SqliteImportCatalog::new(pool, [19; 32]);
    let mut grant = CatalogReadGrant {
        client_id: "first-client".into(),
        grant_version: 1,
        destinations: DestinationGrant {
            catalog_id: preference.catalog_id,
            root_ids: vec!["root".into()],
        },
        can_read_folders: true,
        can_read_tags: false,
    };

    let roots = catalog.list_folders(&grant, None, request(None)).await?;
    assert_eq!(roots.items.len(), 1);
    assert_eq!(roots.items[0].id, "root");
    assert!(roots.items[0].has_children);
    let public = serde_json::to_value(&roots.items[0]).map_err(serialization_error)?;
    assert!(public.get("path").is_none());
    for parent in ["private", "missing"] {
        assert!(matches!(
            catalog
                .list_folders(&grant, Some(parent), request(None))
                .await,
            Err(AppError::NotFound(_))
        ));
    }
    let first = catalog
        .list_folders(&grant, Some("root"), request(None))
        .await?;
    assert_eq!(first.items[0].id, "alpha");
    let cursor = first.next_cursor.as_deref();
    assert!(cursor.is_some());
    let second = catalog
        .list_folders(&grant, Some("root"), request(cursor))
        .await?;
    assert_eq!(second.items[0].id, "beta");
    assert!(second.next_cursor.is_none());
    assert!(matches!(
        catalog.list_folders(&grant, None, request(cursor)).await,
        Err(AppError::CursorExpired)
    ));
    grant.client_id = "second-client".into();
    assert!(matches!(
        catalog
            .list_folders(&grant, Some("root"), request(cursor))
            .await,
        Err(AppError::CursorExpired)
    ));
    grant.client_id = "first-client".into();
    grant.grant_version = 2;
    assert!(matches!(
        catalog
            .list_folders(&grant, Some("root"), request(cursor))
            .await,
        Err(AppError::CursorExpired)
    ));
    grant.grant_version = 1;
    let tampered = cursor.map(|token| format!("9{}", &token[1..]));
    assert!(matches!(
        catalog
            .list_folders(&grant, Some("root"), request(tampered.as_deref()))
            .await,
        Err(AppError::CursorExpired)
    ));
    sqlx::query("UPDATE folders SET parent_id = 'private' WHERE id = 'beta'")
        .execute(pool)
        .await?;
    assert!(matches!(
        catalog
            .list_folders(&grant, Some("root"), request(cursor))
            .await,
        Err(AppError::CursorExpired)
    ));

    assert!(matches!(
        catalog.list_tags(&grant, None, request(None)).await,
        Err(AppError::Forbidden(_))
    ));
    grant.can_read_tags = true;
    let tags = catalog
        .list_tags(&grant, Some(" PHOTO "), request(None))
        .await?;
    assert_eq!(tags.items[0].id, "leaf");
    assert_eq!(tags.items[0].labels, ["Reference", "Photo"]);
    let next = catalog
        .list_tags(&grant, Some("PHOTO"), request(tags.next_cursor.as_deref()))
        .await?;
    assert_eq!(next.items[0].id, "literal");
    assert!(next.next_cursor.is_none());
    let literal = catalog.list_tags(&grant, Some("%"), request(None)).await?;
    assert_eq!(literal.items[0].id, "literal");
    assert!(literal.next_cursor.is_none());
    sqlx::query("UPDATE tags SET name = 'Archive' WHERE id = 'parent'")
        .execute(pool)
        .await?;
    assert!(matches!(
        catalog
            .list_tags(&grant, Some("PHOTO"), request(tags.next_cursor.as_deref()))
            .await,
        Err(AppError::CursorExpired)
    ));
    grant.destinations.catalog_id = "unknown-catalog".into();
    assert!(matches!(
        catalog.list_tags(&grant, None, request(None)).await,
        Err(AppError::NotFound(_))
    ));
    database.close().await;
    Ok(())
}
