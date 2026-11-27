use super::DuplicateCommandService;
use crate::{
    core::{
        ledger::{command::LedgerCommand, port::TransactionalAssetLedger},
        models::{
            DuplicateCandidate, DuplicateGroup, DuplicateGroupStatus, DuplicateGroupType,
            DuplicateResolutionAction, MetadataMergePayload,
        },
        plugins::{
            asset_data::{
                PluginAssetDataLedger, PluginAssetFieldKey, PluginBooleanField, PluginBooleanWrite,
            },
            export::PluginAnnotationReader,
        },
        repository::{AssetQueryHandler, DuplicatesRepository},
        trash::resolve_trash_path,
        AppError,
    },
    infra::database::{
        duplicates::SqliteDuplicatesRepository, plugin_export::SqlitePluginAnnotationReader,
    },
    processing::workers::test_support::{ProcessingFixture, TestResult},
};
use futures_util::TryStreamExt;
use std::sync::Arc;

fn annotation_key(asset_id: &str, field_id: &str) -> PluginAssetFieldKey {
    PluginAssetFieldKey {
        asset_id: asset_id.into(),
        field_id: field_id.into(),
        plugin_id: "org.example.review".into(),
        publisher_fingerprint: [3; 32],
    }
}

fn annotation(value: bool) -> PluginBooleanField {
    PluginBooleanField {
        schema_version: 1,
        revision: 1,
        value,
    }
}

#[tokio::test]
async fn duplicate_merge_keeps_each_plugin_annotation_through_trash_undo_and_purge() -> TestResult {
    let fixture = ProcessingFixture::new().await?;
    let repository = Arc::new(SqliteDuplicatesRepository::new(fixture.pool.clone()));
    let rule = repository
        .get_active_rule_set()
        .await?
        .ok_or("Missing duplicate rule")?;
    let original = fixture.data.path().join("discarded.png");
    let kept_asset_ids = vec!["kept".into()];
    let kept_field = annotation_key("kept", "reviewed");
    let discarded_field = annotation_key("discarded", "reviewed");
    let discarded_only_field = annotation_key("discarded", "archival");
    for asset_id in ["kept", "discarded"] {
        fixture.asset(asset_id, None).await?;
        tokio::fs::write(
            fixture.data.path().join(format!("{asset_id}.png")),
            b"same image",
        )
        .await?;
    }
    let timestamp = chrono::Utc::now();
    repository
        .save_group(DuplicateGroup {
            id: "group".into(),
            rule_set_id: rule.id,
            group_type: DuplicateGroupType::Exact,
            canonical_asset_id: Some("kept".into()),
            confidence: 1.0,
            status: DuplicateGroupStatus::Open,
            candidate_count: 2,
            created_at: timestamp,
            updated_at: timestamp,
        })
        .await?;
    for asset_id in ["kept", "discarded"] {
        repository
            .save_candidate(DuplicateCandidate {
                group_id: "group".into(),
                asset_id: asset_id.into(),
                score: 1.0,
                reasons: "[]".into(),
                is_selected: false,
            })
            .await?;
    }
    let ledger = fixture.ledger.clone();
    for (field, value) in [
        (&kept_field, false),
        (&discarded_field, true),
        (&discarded_only_field, true),
    ] {
        ledger
            .write_plugin_boolean(
                field,
                PluginBooleanWrite {
                    schema_version: 1,
                    expected_revision: 0,
                    value,
                },
            )
            .await?;
    }
    let service = DuplicateCommandService::new(
        repository,
        ledger.clone(),
        fixture.event_bus.clone(),
        fixture.queries.clone(),
        fixture.registry.clone(),
        fixture.processing.clone(),
    );

    service
        .apply_metadata_merge(
            &kept_asset_ids,
            MetadataMergePayload {
                rating: Some(4),
                is_favorite: None,
                notes: None,
                tags_to_add: vec![],
                technical_payload_override: None,
            },
        )
        .await?;
    let kept = fixture
        .queries
        .get_by_id("kept")
        .await?
        .ok_or("Missing kept asset")?;
    assert_eq!(kept.rating, Some(4));
    assert_eq!(
        ledger.read_plugin_boolean(&kept_field).await?,
        Some(annotation(false))
    );
    assert_eq!(
        ledger
            .read_plugin_boolean(&annotation_key("kept", "archival"))
            .await?,
        None
    );

    service
        .resolve_group(
            "group",
            DuplicateResolutionAction::CustomSelection,
            Some(kept_asset_ids.clone()),
        )
        .await?;
    assert!(!tokio::fs::try_exists(&original).await?);
    assert!(matches!(
        ledger.read_plugin_boolean(&discarded_field).await,
        Err(AppError::NotFound(_))
    ));
    let export_reader = SqlitePluginAnnotationReader(&fixture.pool);
    let exported: Vec<_> = export_reader.annotations().try_collect().await?;
    assert_eq!(
        exported
            .iter()
            .filter(|field| field.asset_id == "discarded")
            .count(),
        2
    );

    service.undo_duplicate_resolution("group").await?;
    assert_eq!(tokio::fs::read(&original).await?, b"same image");
    assert_eq!(
        ledger.read_plugin_boolean(&discarded_field).await?,
        Some(annotation(true))
    );
    assert_eq!(
        ledger.read_plugin_boolean(&discarded_only_field).await?,
        Some(annotation(true))
    );

    service
        .resolve_group(
            "group",
            DuplicateResolutionAction::CustomSelection,
            Some(kept_asset_ids),
        )
        .await?;
    let discarded = fixture
        .queries
        .get_by_id("discarded")
        .await?
        .ok_or("Missing trashed asset")?;
    assert!(discarded.deleted_at.is_some());
    let trash_path = resolve_trash_path(
        fixture.data.path(),
        &discarded.id,
        &discarded.path,
        discarded.deleted_at.as_ref(),
    );
    assert!(tokio::fs::try_exists(&trash_path).await?);
    ledger
        .execute(LedgerCommand::DeleteAsset {
            asset_id: Some(discarded.id),
            path: Some(trash_path.clone()),
            physical_delete: true,
        })
        .await?;
    assert!(!tokio::fs::try_exists(&trash_path).await?);
    let exported: Vec<_> = export_reader.annotations().try_collect().await?;
    assert_eq!(exported.len(), 1);
    assert_eq!(exported[0].asset_id, "kept");
    assert_eq!(
        ledger.read_plugin_boolean(&kept_field).await?,
        Some(annotation(false))
    );
    Ok(())
}
