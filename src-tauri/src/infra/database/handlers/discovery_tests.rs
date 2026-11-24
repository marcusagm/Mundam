//! Integration coverage for migrations, ledger events, search projections and rollback.
use crate::core::events::{AppEventBus, DomainEvent};
use crate::core::formats::registry::FormatRegistry;
use crate::core::ledger::command::{LedgerCommand, UpdateAssetDiscoveryPayload};
use crate::core::ledger::port::TransactionalAssetLedger;
use crate::core::models::asset::{AssetFilter, PageParams};
use crate::core::models::search::{LogicalOperator, SearchCriterion, SearchGroup, SearchItem};
use crate::core::path_guard::LedgerPathGuard;
use crate::infra::database::search_builder::build_search_where_clause;
use crate::infra::database::{
    ledger::SqliteAssetLedger, manager::DbManager, query_handlers::asset_queries,
};
use crate::infra::events::TokioEventBus;
use std::sync::Arc;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn discovery_metadata_round_trip_search_events_and_rollback() -> TestResult {
    let directory = tempfile::tempdir()?;
    let database = DbManager::new(&directory.path().join("discovery.db")).await?;
    let pool = database.pool();
    sqlx::query("INSERT INTO assets(id,name,path,state,format_type,family,file_size) VALUES ('asset','reference.png','/reference.png','Indexed','png','IMAGE',1), ('empty','local.png','/local.png','Indexed','png','IMAGE',1)").execute(pool).await?;
    let bus = Arc::new(TokioEventBus::new());
    let mut events = bus.subscribe();
    let ledger = SqliteAssetLedger::new(
        pool.clone(),
        bus,
        directory.path().to_path_buf(),
        Arc::new(LedgerPathGuard::new()),
    );
    let asset = ledger
        .execute(LedgerCommand::UpdateAssetDiscovery(
            UpdateAssetDiscoveryPayload {
                asset_id: "asset".into(),
                keywords: Some(vec![
                    " nature, Nature ".into(),
                    "concept art".into(),
                    "".into(),
                ]),
                source_url: Some(" https://example.com/reference ".into()),
            },
        ))
        .await?;
    assert_eq!(asset.keywords, vec!["nature", "concept art"]);
    assert_eq!(
        asset.source_url.as_deref(),
        Some("https://example.com/reference")
    );
    assert!(
        matches!(events.try_recv()?, DomainEvent::AssetMetadataUpdated { asset_id } if asset_id == "asset")
    );
    let registry = FormatRegistry::new();
    for fuzzy in [false, true] {
        let filter = AssetFilter {
            search_query: Some("nature".into()),
            search_fuzzy: Some(fuzzy),
            ..Default::default()
        };
        let rows =
            asset_queries::list_paginated(pool, &registry, filter.clone(), PageParams::default())
                .await?;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].keywords, asset.keywords);
        assert_eq!(
            asset_queries::get_asset_count(pool, &registry, filter).await?,
            1
        );
    }
    for (operator, value, count) in [
        ("equals", "NATURE", 1),
        ("contains", "concept", 1),
        ("starts_with", "nat", 1),
        ("ends_with", "art", 1),
        ("not_contains", "nature", 1),
    ] {
        let group = SearchGroup {
            id: "root".into(),
            logical_operator: LogicalOperator::And,
            items: vec![SearchItem::Criterion(SearchCriterion {
                id: "rule".into(),
                key: "keywords".into(),
                operator: operator.into(),
                value: serde_json::json!(value),
            })],
        };
        let mut query = sqlx::QueryBuilder::new("SELECT COUNT(*) FROM assets a WHERE ");
        build_search_where_clause(&group, &mut query, &registry);
        assert_eq!(
            query.build_query_scalar::<i64>().fetch_one(pool).await?,
            count
        );
    }
    let rejected = ledger
        .execute(LedgerCommand::UpdateAssetDiscovery(
            UpdateAssetDiscoveryPayload {
                asset_id: "asset".into(),
                keywords: Some(vec!["must not persist".into()]),
                source_url: Some("javascript:alert(1)".into()),
            },
        ))
        .await;
    assert!(rejected.is_err());
    assert!(events.try_recv().is_err());
    let persisted = asset_queries::get_asset_by_id(pool, &registry, "asset").await?;
    assert_eq!(persisted.keywords, asset.keywords);
    let cleared = ledger
        .execute(LedgerCommand::UpdateAssetDiscovery(
            UpdateAssetDiscoveryPayload {
                asset_id: "asset".into(),
                keywords: Some(vec![]),
                source_url: Some(String::new()),
            },
        ))
        .await?;
    assert!(cleared.keywords.is_empty());
    assert!(cleared.source_url.is_none());
    let indexed: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM assets_fts WHERE assets_fts MATCH 'nature'")
            .fetch_one(pool)
            .await?;
    assert_eq!(indexed, 0);
    database.close().await;
    Ok(())
}
