//! Transaction-level regression tests for taxonomy import and color behavior.

use super::{tag_import::handle_import, tags_handler::handle_update_tag};
use crate::core::error::AppResult;
use crate::core::ledger::command::{ImportTagEntry, ImportTagsPayload, UpdateTagPayload};
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};

async fn database() -> AppResult<SqlitePool> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;
    sqlx::raw_sql("PRAGMA foreign_keys = ON;
        CREATE TABLE tags (id TEXT PRIMARY KEY, name TEXT UNIQUE NOT NULL, color TEXT,
            parent_id TEXT REFERENCES tags(id) ON DELETE SET NULL, order_index INTEGER NOT NULL DEFAULT 0,
            inherits_color BOOLEAN NOT NULL DEFAULT TRUE);
        CREATE TABLE asset_tags (asset_id TEXT, tag_id TEXT REFERENCES tags(id) ON DELETE CASCADE);
        CREATE TABLE asset_operations_log (id TEXT, operation_type TEXT, asset_id TEXT, payload TEXT,
            status TEXT, error_note TEXT, created_at TEXT);
    ").execute(&pool).await?;
    Ok(pool)
}

fn entry(name: &str, parent_index: Option<usize>, color: Option<&str>) -> ImportTagEntry {
    ImportTagEntry {
        name: name.into(),
        parent_index,
        color: color.map(str::to_string),
    }
}

fn update(
    id: &str,
    color: Option<&str>,
    inherits_color: Option<bool>,
    parent_id: Option<&str>,
) -> UpdateTagPayload {
    UpdateTagPayload {
        id: id.into(),
        name: None,
        color: color.map(str::to_string),
        parent_id: parent_id.map(str::to_string),
        order_index: None,
        inherits_color,
    }
}

async fn color(pool: &SqlitePool, name: &str) -> AppResult<Option<String>> {
    Ok(sqlx::query_scalar("SELECT color FROM tags WHERE name = ?")
        .bind(name)
        .fetch_one(pool)
        .await?)
}

#[tokio::test]
async fn tag_queries_expose_effective_colors_and_inheritance_consistently() -> AppResult<()> {
    use crate::core::formats::registry::FormatRegistry;
    use crate::infra::database::query_handlers::tags_queries;

    let pool = database().await?;
    let mut transaction = pool.begin().await?;
    handle_import(
        &mut transaction,
        ImportTagsPayload {
            replace_existing: false,
            entries: vec![
                entry("Parent", None, Some("#123456")),
                entry("Child", Some(0), None),
            ],
        },
    )
    .await?;
    transaction.commit().await?;
    sqlx::query("INSERT INTO asset_tags SELECT 'asset', id FROM tags")
        .execute(&pool)
        .await?;
    let registry = FormatRegistry::new();
    let all_tags = tags_queries::list_tags(&pool, &registry).await?;
    let asset_tags = tags_queries::get_tags_for_asset(&pool, &registry, "asset").await?;
    let batch_tags = tags_queries::get_tags_for_assets(&pool, &registry, &["asset".into()]).await?;
    for tags in [&all_tags, &asset_tags, &batch_tags["asset"]] {
        assert_eq!(tags.len(), 2);
        let child = tags.iter().find(|tag| tag.name == "Child");
        assert!(
            child.is_some_and(|tag| tag.inherits_color && tag.color.as_deref() == Some("#123456"))
        );
        let parent = tags.iter().find(|tag| tag.name == "Parent");
        assert!(parent
            .is_some_and(|tag| !tag.inherits_color && tag.color.as_deref() == Some("#123456")));
    }
    Ok(())
}

#[tokio::test]
async fn inherited_colors_follow_changes_custom_boundaries_and_reparenting() -> AppResult<()> {
    let pool = database().await?;
    let mut transaction = pool.begin().await?;
    handle_import(
        &mut transaction,
        ImportTagsPayload {
            replace_existing: false,
            entries: vec![
                entry("Parent", None, Some("#123456")),
                entry("Child", Some(0), None),
                entry("Grandchild", Some(1), None),
                entry("Custom", Some(0), Some("#123456")),
                entry("Custom child", Some(3), None),
                entry("Other", None, Some("#abcdef")),
            ],
        },
    )
    .await?;
    transaction.commit().await?;
    let parent: String = sqlx::query_scalar("SELECT id FROM tags WHERE name = 'Parent'")
        .fetch_one(&pool)
        .await?;
    let custom: String = sqlx::query_scalar("SELECT id FROM tags WHERE name = 'Custom'")
        .fetch_one(&pool)
        .await?;
    let child: String = sqlx::query_scalar("SELECT id FROM tags WHERE name = 'Child'")
        .fetch_one(&pool)
        .await?;
    let other: String = sqlx::query_scalar("SELECT id FROM tags WHERE name = 'Other'")
        .fetch_one(&pool)
        .await?;
    let mut transaction = pool.begin().await?;
    handle_update_tag(
        &mut transaction,
        update(&parent, Some("#654321"), None, None),
    )
    .await?;
    transaction.commit().await?;
    assert_eq!(
        color(&pool, "Grandchild").await?.as_deref(),
        Some("#654321")
    );
    assert_eq!(
        color(&pool, "Custom child").await?.as_deref(),
        Some("#123456")
    );
    let mut transaction = pool.begin().await?;
    handle_update_tag(&mut transaction, update(&custom, None, Some(true), None)).await?;
    handle_update_tag(&mut transaction, update(&child, None, None, Some(&other))).await?;
    transaction.commit().await?;
    assert_eq!(
        color(&pool, "Custom child").await?.as_deref(),
        Some("#654321")
    );
    assert_eq!(
        color(&pool, "Grandchild").await?.as_deref(),
        Some("#abcdef")
    );
    let mut transaction = pool.begin().await?;
    assert!(
        handle_update_tag(&mut transaction, update(&other, None, None, Some(&child)))
            .await
            .is_err()
    );
    transaction.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn failed_replacement_restores_tags_and_asset_associations() -> AppResult<()> {
    let pool = database().await?;
    sqlx::raw_sql(
        "INSERT INTO tags (id, name) VALUES ('old', 'Old');
        INSERT INTO asset_tags VALUES ('asset', 'old');
        CREATE TRIGGER reject_tag BEFORE INSERT ON tags WHEN NEW.name = 'Rejected'
        BEGIN SELECT RAISE(ABORT, 'Import failure'); END;
    ",
    )
    .execute(&pool)
    .await?;
    let mut transaction = pool.begin().await?;
    assert!(handle_import(
        &mut transaction,
        ImportTagsPayload {
            replace_existing: true,
            entries: vec![entry("Accepted", None, None), entry("Rejected", None, None)]
        }
    )
    .await
    .is_err());
    transaction.rollback().await?;
    let names: Vec<String> = sqlx::query_scalar("SELECT name FROM tags")
        .fetch_all(&pool)
        .await?;
    assert_eq!(names, ["Old"]);
    let associations: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM asset_tags")
        .fetch_one(&pool)
        .await?;
    assert_eq!(associations, 1);
    Ok(())
}

#[tokio::test]
async fn migration_preserves_explicit_colors_and_inherits_previously_uncolored_tags(
) -> AppResult<()> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;
    sqlx::raw_sql(
        "CREATE TABLE tags (id TEXT PRIMARY KEY, name TEXT, color TEXT, parent_id TEXT);
        INSERT INTO tags VALUES ('root', 'Root', '#abc', NULL),
            ('child', 'Child', NULL, 'root'), ('custom', 'Custom', '#abc', 'root');
    ",
    )
    .execute(&pool)
    .await?;
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20261105010000_tag_color_inheritance.sql"
    ))
    .execute(&pool)
    .await?;
    let colors: Vec<(String, Option<String>, bool)> =
        sqlx::query_as("SELECT name, color, inherits_color FROM tags ORDER BY name")
            .fetch_all(&pool)
            .await?;
    assert_eq!(
        colors,
        vec![
            ("Child".into(), Some("#abc".into()), true),
            ("Custom".into(), Some("#abc".into()), false),
            ("Root".into(), Some("#abc".into()), false)
        ]
    );
    Ok(())
}

#[tokio::test]
async fn imports_preserve_associations_order_and_rollback_conflicts() -> AppResult<()> {
    let pool = database().await?;
    let mut transaction = pool.begin().await?;
    handle_import(
        &mut transaction,
        ImportTagsPayload {
            replace_existing: false,
            entries: vec![entry("Old", None, Some("#abc"))],
        },
    )
    .await?;
    transaction.commit().await?;
    sqlx::query("INSERT INTO asset_tags SELECT 'asset', id FROM tags")
        .execute(&pool)
        .await?;
    let mut transaction = pool.begin().await?;
    handle_import(
        &mut transaction,
        ImportTagsPayload {
            replace_existing: false,
            entries: vec![
                entry("Z first", None, None),
                entry("Old", None, Some("#def")),
                entry("A last", None, None),
            ],
        },
    )
    .await?;
    transaction.commit().await?;
    let names: Vec<String> = sqlx::query_scalar("SELECT name FROM tags ORDER BY order_index")
        .fetch_all(&pool)
        .await?;
    assert_eq!(names, ["Z first", "Old", "A last"]);
    let associations: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM asset_tags")
        .fetch_one(&pool)
        .await?;
    assert_eq!(associations, 1);
    let mut transaction = pool.begin().await?;
    let result = handle_import(
        &mut transaction,
        ImportTagsPayload {
            replace_existing: false,
            entries: vec![entry("New", None, None), entry("Old", Some(0), None)],
        },
    )
    .await;
    assert!(result.is_err());
    transaction.rollback().await?;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tags")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 3);
    let mut transaction = pool.begin().await?;
    handle_import(
        &mut transaction,
        ImportTagsPayload {
            replace_existing: true,
            entries: vec![entry("Replacement", None, None)],
        },
    )
    .await?;
    transaction.commit().await?;
    let associations: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM asset_tags")
        .fetch_one(&pool)
        .await?;
    assert_eq!(associations, 0);
    let names: Vec<String> = sqlx::query_scalar("SELECT name FROM tags")
        .fetch_all(&pool)
        .await?;
    assert_eq!(names, ["Replacement"]);
    Ok(())
}
