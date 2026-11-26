//! Ensures package review, SQLite admission and reopening agree on the stored identity.

use super::*;
use crate::{
    core::plugins::{InstallApproval, PluginReplacementKind},
    feature::plugins::PluginService,
};
use mundam_plugin_sdk::package::{sign_package, signing_key_fingerprint};
use serde_json::{json, Value};
use std::sync::Arc;

const MANIFEST: &[u8] =
    include_bytes!("../../../../../sdk/plugins/v1/examples/reference-review/manifest.json");
const ICON: &[u8] =
    include_bytes!("../../../../../sdk/plugins/v1/examples/reference-review/assets/review.png");

fn fixture_key(seed_byte: u8) -> Vec<u8> {
    // PKCS#8 v1 wrapping a deterministic seed reserved for test packages.
    let mut key = vec![
        0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04,
        0x20,
    ];
    key.extend_from_slice(&[seed_byte; 32]);
    key
}

fn package(
    namespace: &str,
    version: &str,
    minimum_api: &str,
    seed: u8,
) -> Result<(PublisherAnchor, Vec<u8>), Box<dyn std::error::Error>> {
    let key = fixture_key(seed);
    let mut declaration: Value = serde_json::from_slice(MANIFEST)?;
    declaration["publisher"] = json!(namespace);
    declaration["id"] = json!(format!("{namespace}.reference-review"));
    declaration["version"] = json!(version);
    declaration["engines"]["mundamApi"] = json!(minimum_api);
    let manifest = serde_json::to_vec(&declaration)?;
    let publisher = PublisherAnchor {
        namespace: namespace.into(),
        fingerprint: signing_key_fingerprint(&key)?,
    };
    Ok((publisher, sign_package(&manifest, ICON, &key)?))
}

#[tokio::test]
async fn inspector_projects_only_valid_and_current_catalog_grants(
) -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let registry =
        Arc::new(SqlitePluginRegistry::open(&directory.path().join("plugins.sqlite")).await?);
    let service = PluginService::new(registry.clone());
    let catalog = "catalog-a";
    let (publisher, bytes) = package("org.example", "1.0.0", ">=1.0.0 <2.0.0", 19)?;
    service.trust_publisher(publisher.clone()).await?;
    let preview = service.preview(bytes.clone()).await?;
    let first = service
        .install(
            bytes,
            InstallApproval {
                package_digest: preview.package_digest,
            },
        )
        .await?;
    assert!(service.presentation(catalog).await?.sections.is_empty());
    let approval = service.review_activation(catalog.into(), &first.id).await?;
    service.enable(&approval, vec![]).await?;
    let presentation = service.presentation(catalog).await?;
    assert_eq!(presentation.screens[0].plugin_id, first.id);
    assert_eq!(presentation.screens[0].screen_id, "review-guide");
    assert_eq!(presentation.commands[0].plugin_id, first.id);
    assert_eq!(presentation.commands[0].command_id, "open-review-guide");
    let sections = presentation.sections;
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].plugin_id, first.id);
    assert_eq!(sections[0].section_id, "review");
    assert_eq!(
        sections[0].viewport_action_label.as_deref(),
        Some("Show review in Inspector")
    );
    assert_eq!(
        sections[0].sidebar_link_label.as_deref(),
        Some("Open review details")
    );
    assert_eq!(
        sections[0].title_bar_action_label.as_deref(),
        Some("Show review in Inspector")
    );
    assert_eq!(sections[0].fields[0].field_id, "reviewed");
    assert!(!sections[0].fields[0].default_value);
    assert!(service.presentation("catalog-b").await?.sections.is_empty());
    let safe_mode = PluginService::new(registry.clone()).with_safe_mode(true);
    assert!(safe_mode.presentation(catalog).await?.sections.is_empty());
    assert!(safe_mode.presentation(catalog).await?.screens.is_empty());
    assert!(safe_mode.presentation(catalog).await?.commands.is_empty());
    service.disable(catalog, &first.id).await?;
    assert!(service.presentation(catalog).await?.sections.is_empty());
    assert!(service.presentation(catalog).await?.screens.is_empty());
    assert!(service.presentation(catalog).await?.commands.is_empty());
    service.enable(&approval, vec![]).await?;

    let (other_publisher, bytes) = package("org.other", "1.0.0", ">=1.0.0 <2.0.0", 21)?;
    service.trust_publisher(other_publisher).await?;
    let preview = service.preview(bytes.clone()).await?;
    let other = service
        .install(
            bytes,
            InstallApproval {
                package_digest: preview.package_digest,
            },
        )
        .await?;
    let other_approval = service.review_activation(catalog.into(), &other.id).await?;
    service.enable(&other_approval, vec![]).await?;
    assert_eq!(service.presentation(catalog).await?.sections.len(), 2);
    sqlx::query("UPDATE plugin_catalog_activations SET permissions = '[\"selection.read\"]' WHERE plugin_id = ?")
        .bind(&first.id).execute(&registry.0).await?;
    assert_eq!(
        service.presentation(catalog).await?.sections[0].plugin_id,
        other.id
    );
    service.enable(&approval, vec![]).await?;
    sqlx::query("UPDATE installed_plugins SET package = X'01' WHERE id = ?")
        .bind(&first.id)
        .execute(&registry.0)
        .await?;
    let sections = service.presentation(catalog).await?.sections;
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].plugin_id, other.id);
    service.revoke_publisher(publisher).await?;
    assert_eq!(service.presentation(catalog).await?.sections.len(), 1);
    Ok(())
}

#[tokio::test]
async fn swaps_signed_versions_atomically_and_consumes_the_rollback_slot(
) -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let registry =
        Arc::new(SqlitePluginRegistry::open(&directory.path().join("plugins.sqlite")).await?);
    let service = PluginService::new(registry.clone());
    let tuple = package("org.example", "1.0.0", ">=1.0.0 <2.0.0", 14)?;
    let (publisher, _) = tuple;
    let key = fixture_key(14);
    let mut first_declaration: Value = serde_json::from_slice(MANIFEST)?;
    first_declaration["permissions"] = json!(["selection.read", "assetData.own.read"]);
    first_declaration["optionalPermissions"] = json!(["assetData.own.write"]);
    first_declaration["contributes"]["inspectorSections"][0]["fields"][0]["readOnly"] = json!(true);
    let first_bytes = sign_package(&serde_json::to_vec(&first_declaration)?, ICON, &key)?;
    let (_, second_bytes) = package("org.example", "1.1.0", ">=1.0.0 <2.0.0", 14)?;
    let (_, unsupported_bytes) = package("org.example", "2.0.0", ">=2.0.0 <3.0.0", 14)?;
    let (_, equal_precedence) = package("org.example", "1.0.0+repack", ">=1.0.0 <2.0.0", 14)?;
    service.trust_publisher(publisher.clone()).await?;
    let initial = service.review_package_change(first_bytes.clone()).await?;
    let id = initial.preview.plugin_id.clone();
    let first = service
        .install(
            first_bytes.clone(),
            InstallApproval {
                package_digest: initial.preview.package_digest,
            },
        )
        .await?;
    let approval = service.review_activation("catalog-a".into(), &id).await?;
    service.enable(&approval, vec![]).await?;
    let other_approval = service.review_activation("catalog-b".into(), &id).await?;
    service.enable(&other_approval, vec![]).await?;
    assert!(service
        .review_package_change(equal_precedence)
        .await
        .is_err());

    let unsupported = service
        .review_package_change(unsupported_bytes.clone())
        .await?;
    assert!(!unsupported.preview.compatible);
    assert!(service
        .replace_package(
            unsupported_bytes,
            InstallApproval {
                package_digest: unsupported.preview.package_digest,
            },
            unsupported
                .replacement
                .as_ref()
                .ok_or("missing replacement")?
        )
        .await
        .is_err());
    assert!(service.list_installed("catalog-a").await?[0].enabled);
    assert!(registry.rollback_package(&id).await?.is_none());

    let change = service.review_package_change(second_bytes.clone()).await?;
    let reviewed = change.replacement.as_ref().ok_or("missing update")?;
    assert_eq!(reviewed.kind, PluginReplacementKind::Update);
    assert_eq!(reviewed.added_permissions, [Permission::OwnDataWrite]);
    let update = InstallApproval {
        package_digest: change.preview.package_digest,
    };
    let (left, right) = tokio::join!(
        service.replace_package(second_bytes.clone(), update, reviewed),
        service.replace_package(second_bytes.clone(), update, reviewed)
    );
    let updated = match (left, right) {
        (Ok(installed), Err(_)) | (Err(_), Ok(installed)) => installed,
        _ => return Err("exactly one reviewed update must commit".into()),
    };
    assert_eq!(updated.version, "1.1.0");
    assert_eq!(updated.rollback_version.as_deref(), Some("1.0.0"));
    assert!(!service.list_installed("catalog-a").await?[0].approved);
    assert!(!service.list_installed("catalog-b").await?[0].approved);
    assert!(service
        .review_package_change(first_bytes.clone())
        .await
        .is_err());

    let (rollback, bytes) = service.review_rollback(&id).await?;
    assert_eq!(bytes, first_bytes);
    let restored = service
        .replace_package(
            bytes,
            InstallApproval {
                package_digest: rollback.preview.package_digest,
            },
            rollback.replacement.as_ref().ok_or("missing rollback")?,
        )
        .await?;
    assert_eq!(restored.version, "1.0.0");
    assert!(restored.rollback_version.is_none());
    assert_ne!(restored.package_revision, first.package_revision);
    assert!(service.enable(&approval, vec![]).await.is_err());
    assert!(service
        .remove_package(&id, &first.package_revision)
        .await
        .is_err());
    assert!(service.review_rollback(&id).await.is_err());

    let change = service.review_package_change(second_bytes.clone()).await?;
    let current = service
        .replace_package(
            second_bytes,
            update,
            change.replacement.as_ref().ok_or("missing update")?,
        )
        .await?;
    let (pending, rollback_bytes) = service.review_rollback(&id).await?;
    service.revoke_publisher(publisher).await?;
    assert!(service.review_rollback(&id).await.is_err());
    assert!(service
        .replace_package(
            rollback_bytes,
            InstallApproval {
                package_digest: pending.preview.package_digest,
            },
            pending.replacement.as_ref().ok_or("missing rollback")?
        )
        .await
        .is_err());
    assert_eq!(
        service.list_installed("catalog-a").await?[0].version,
        "1.1.0"
    );
    service
        .remove_package(&id, &current.package_revision)
        .await?;
    assert!(service.list_installed("catalog-a").await?.is_empty());
    assert!(registry.rollback_package(&id).await?.is_none());
    assert!(service.list_publishers().await?[0].revoked);
    Ok(())
}

#[tokio::test]
async fn installs_only_reviewed_compatible_bytes_once_and_keeps_them_after_reopen(
) -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let database_path = directory.path().join("plugins.sqlite");
    let registry = Arc::new(SqlitePluginRegistry::open(&database_path).await?);
    let service = PluginService::new(registry.clone());
    let (publisher, bytes) = package("org.example", "1.0.0", ">=1.0.0 <2.0.0", 9)?;
    let (_, replacement) = package("org.example", "1.0.1", ">=1.0.0 <2.0.0", 9)?;
    let (_, incompatible) = package("org.example", "2.0.0", ">=2.0.0 <3.0.0", 9)?;

    assert!(matches!(
        service.preview(bytes.clone()).await,
        Err(AppError::Forbidden(_))
    ));
    service.trust_publisher(publisher.clone()).await?;
    service.trust_publisher(publisher.clone()).await?;
    let preview = service.preview(bytes.clone()).await?;
    assert_eq!(preview.publisher, publisher);
    assert_eq!(preview.icon_png, ICON);
    assert_eq!(preview.required_permissions.len(), 3);
    assert!(preview.compatible);
    let approval = InstallApproval {
        package_digest: preview.package_digest,
    };

    assert!(matches!(
        service.install(replacement.clone(), approval).await,
        Err(AppError::ValidationFailed(_))
    ));
    let unavailable = service.preview(incompatible.clone()).await?;
    assert!(!unavailable.compatible);
    assert!(matches!(
        service
            .install(
                incompatible,
                InstallApproval {
                    package_digest: unavailable.package_digest
                }
            )
            .await,
        Err(AppError::ValidationFailed(_))
    ));
    assert!(service.list_installed("catalog-a").await?.is_empty());

    let (first, retried) = tokio::join!(
        service.install(bytes.clone(), approval),
        service.install(bytes.clone(), approval),
    );
    let installed = first?;
    assert_eq!(installed, retried?);
    assert!(!installed.enabled);
    assert_eq!(service.list_installed("catalog-a").await?, [installed]);
    let replacement_approval = InstallApproval {
        package_digest: service.preview(replacement.clone()).await?.package_digest,
    };
    assert!(matches!(
        service.install(replacement, replacement_approval).await,
        Err(AppError::ValidationFailed(_))
    ));

    registry.0.close().await;
    let reopened = Arc::new(SqlitePluginRegistry::open(&database_path).await?);
    let service = PluginService::new(reopened.clone());
    assert!(service.preview(bytes.clone()).await?.compatible);
    assert_eq!(service.list_installed("catalog-a").await?.len(), 1);
    let persisted = sqlx::query_scalar::<_, Vec<u8>>("SELECT package FROM installed_plugins")
        .fetch_one(&reopened.0)
        .await?;
    assert_eq!(persisted, bytes);
    Ok(())
}

#[tokio::test]
async fn namespace_and_key_cannot_be_reassigned_and_revocation_beats_a_stale_preview(
) -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let database_path = directory.path().join("plugins.sqlite");
    let registry = Arc::new(SqlitePluginRegistry::open(&database_path).await?);
    let service = PluginService::new(registry.clone());
    let (publisher, bytes) = package("org.example", "1.0.0", ">=1.0.0 <2.0.0", 9)?;
    let (other_key, _) = package("org.example", "1.0.0", ">=1.0.0 <2.0.0", 7)?;
    let (other_namespace, other_package) = package("org.different", "1.0.0", ">=1.0.0 <2.0.0", 9)?;
    service.trust_publisher(publisher.clone()).await?;
    assert!(service.trust_publisher(other_key).await.is_err());
    assert!(service.trust_publisher(other_namespace).await.is_err());
    assert!(matches!(
        service.preview(other_package).await,
        Err(AppError::ValidationFailed(_))
    ));

    let preview = service.preview(bytes.clone()).await?;
    let approval = InstallApproval {
        package_digest: preview.package_digest,
    };
    service.install(bytes.clone(), approval).await?;
    let activation = service
        .review_activation("catalog-a".into(), &preview.plugin_id)
        .await?;
    service.enable(&activation, vec![]).await?;
    service.revoke_publisher(publisher.clone()).await?;
    service.revoke_publisher(publisher.clone()).await?;
    assert!(!service.list_installed("catalog-a").await?[0].enabled);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM plugin_catalog_activations")
            .fetch_one(&registry.0)
            .await?,
        0
    );
    assert!(registry
        .enable(&activation, &activation.required_permissions)
        .await
        .is_err());
    assert!(matches!(
        service.install(bytes.clone(), approval).await,
        Err(AppError::Forbidden(_))
    ));
    // Exercise the database check after the application has already verified a package.
    assert!(matches!(
        registry
            .install(PluginInstallRecord {
                preview: &preview,
                bytes: &bytes
            })
            .await,
        Err(AppError::Forbidden(_))
    ));
    registry.0.close().await;
    let reopened = PluginService::new(Arc::new(SqlitePluginRegistry::open(&database_path).await?));
    assert!(reopened.trust_publisher(publisher).await.is_err());
    assert!(matches!(
        reopened.preview(bytes).await,
        Err(AppError::Forbidden(_))
    ));
    Ok(())
}

#[tokio::test]
async fn catalog_consent_persists_and_safe_mode_blocks_enabling_without_erasing_approval(
) -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("plugins.sqlite");
    let registry = Arc::new(SqlitePluginRegistry::open(&path).await?);
    let service = PluginService::new(registry.clone());
    let key = fixture_key(12);
    let mut declaration: Value = serde_json::from_slice(MANIFEST)?;
    declaration["permissions"] = json!(["selection.read", "assetData.own.read"]);
    declaration["optionalPermissions"] = json!(["assetData.own.write"]);
    declaration["contributes"]["inspectorSections"][0]["fields"][0]["readOnly"] = json!(true);
    let bytes = sign_package(&serde_json::to_vec(&declaration)?, ICON, &key)?;
    let publisher = PublisherAnchor {
        namespace: "org.example".into(),
        fingerprint: signing_key_fingerprint(&key)?,
    };
    service.trust_publisher(publisher).await?;
    let preview = service.preview(bytes.clone()).await?;
    service
        .install(
            bytes,
            InstallApproval {
                package_digest: preview.package_digest,
            },
        )
        .await?;
    let activation = service
        .review_activation("catalog-a".into(), &preview.plugin_id)
        .await?;
    assert!(service
        .enable(&activation, vec![Permission::SelectionRead])
        .await
        .is_err());
    assert!(service
        .enable(&activation, vec![Permission::OwnDataWrite; 2])
        .await
        .is_err());
    service.enable(&activation, vec![]).await?;
    assert!(service.list_installed("catalog-a").await?[0].enabled);
    assert!(!service.list_installed("catalog-b").await?[0].approved);
    let permissions =
        sqlx::query_scalar::<_, String>("SELECT permissions FROM plugin_catalog_activations")
            .fetch_one(&registry.0)
            .await?;
    assert_eq!(
        serde_json::from_str::<Vec<Permission>>(&permissions)?,
        activation.required_permissions
    );
    service
        .enable(&activation, vec![Permission::OwnDataWrite])
        .await?;
    let permissions =
        sqlx::query_scalar::<_, String>("SELECT permissions FROM plugin_catalog_activations")
            .fetch_one(&registry.0)
            .await?;
    assert_eq!(
        serde_json::from_str::<Vec<Permission>>(&permissions)?,
        [
            Permission::SelectionRead,
            Permission::OwnDataRead,
            Permission::OwnDataWrite
        ]
    );
    registry.0.close().await;

    let registry = Arc::new(SqlitePluginRegistry::open(&path).await?);
    let safe = PluginService::new(registry.clone()).with_safe_mode(true);
    let safe_inventory = safe.list_installed("catalog-a").await?;
    assert!(safe_inventory[0].approved);
    assert!(!safe_inventory[0].enabled);
    assert!(safe
        .review_activation("catalog-a".into(), &preview.plugin_id)
        .await
        .is_err());
    assert!(safe.enable(&activation, vec![]).await.is_err());
    let normal = PluginService::new(registry);
    assert!(normal.list_installed("catalog-a").await?[0].enabled);
    safe.disable("catalog-b", &preview.plugin_id).await?;
    assert!(normal.list_installed("catalog-a").await?[0].approved);
    safe.disable("catalog-a", &preview.plugin_id).await?;
    assert!(!normal.list_installed("catalog-a").await?[0].approved);
    Ok(())
}
