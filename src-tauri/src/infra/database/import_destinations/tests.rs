use super::*;
use crate::core::imports::destinations::require_destination;
use crate::infra::database::manager::DbManager;

#[tokio::test]
async fn preference_and_grants_follow_current_catalog_topology() -> AppResult<()> {
    let workspace = tempfile::tempdir()?;
    let root = workspace.path().join("root");
    let outside = workspace.path().join("outside");
    let child = root.join("child");
    tokio::fs::create_dir_all(&child).await?;
    tokio::fs::create_dir(&outside).await?;
    let database = DbManager::new(&workspace.path().join("catalog.sqlite")).await?;
    let destinations = SqliteImportDestinations(database.pool());
    let initial = destinations.preference().await?;
    assert_eq!(initial.folder_id, None);

    for (id, parent, path) in [
        ("root", None, &root),
        ("outside", None, &outside),
        ("child", Some("root"), &child),
        ("mismatch", Some("root"), &outside.join("nested")),
    ] {
        tokio::fs::create_dir_all(path).await?;
        sqlx::query("INSERT INTO folders (id, parent_id, name, path) VALUES (?, ?, ?, ?)")
            .bind(id)
            .bind(parent)
            .bind(id)
            .bind(path.to_string_lossy().as_ref())
            .execute(database.pool())
            .await?;
    }
    let grant = DestinationGrant {
        catalog_id: initial.catalog_id.clone(),
        root_ids: vec!["root".into()],
    };
    let wrong_catalog = DestinationGrant {
        catalog_id: "another-catalog".into(),
        root_ids: vec!["root".into()],
    };
    let saved = destinations
        .set_default(&initial.catalog_id, Some("child"))
        .await?;
    assert_eq!(saved.folder_id.as_deref(), Some("child"));
    assert!(destinations
        .set_default("another-catalog", None)
        .await
        .is_err());
    assert!(destinations
        .set_default(&initial.catalog_id, Some("missing"))
        .await
        .is_err());
    assert_eq!(destinations.preference().await?, saved);
    assert_eq!(
        require_destination(&destinations, &grant, None).await?,
        "child"
    );
    assert_eq!(destinations.resolve(&wrong_catalog, None).await?, None);
    assert_eq!(destinations.resolve(&grant, Some("outside")).await?, None);
    assert_eq!(destinations.resolve(&grant, Some("mismatch")).await?, None);
    assert_eq!(destinations.resolve(&grant, Some("missing")).await?, None);

    sqlx::query("UPDATE folders SET name = 'renamed' WHERE id = 'child'")
        .execute(database.pool())
        .await?;
    assert_eq!(
        require_destination(&destinations, &grant, None).await?,
        "child"
    );
    tokio::fs::rename(&child, root.join("offline")).await?;
    assert_eq!(destinations.resolve(&grant, None).await?, None);
    assert_eq!(destinations.preference().await?, saved);
    tokio::fs::rename(root.join("offline"), &child).await?;

    sqlx::query("UPDATE folders SET parent_id = 'outside' WHERE id = 'child'")
        .execute(database.pool())
        .await?;
    assert_eq!(destinations.resolve(&grant, None).await?, None);
    sqlx::query("DELETE FROM folders WHERE id = 'child'")
        .execute(database.pool())
        .await?;
    assert_eq!(destinations.preference().await?, initial);
    destinations
        .set_default(&initial.catalog_id, Some("root"))
        .await?;
    assert_eq!(
        destinations.set_default(&initial.catalog_id, None).await?,
        initial
    );
    database.close().await;
    Ok(())
}
