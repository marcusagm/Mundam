//! Covers the review ticket and the desktop command state used by Settings.

use super::*;
use crate::{
    bootstrap::AppDirectories,
    delivery::tauri::{admission::IpcAdmission, commands::plugins::*},
    infra::database::manager::DbManager,
};
use mundam_plugin_sdk::package::{sign_package, signing_key_fingerprint};
use serde_json::{json, Value};
use tauri::{test::MockRuntime, Manager, WebviewWindow, WebviewWindowBuilder};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn signed_fixture(version: &str) -> Result<(PublisherAnchor, Vec<u8>), Box<dyn std::error::Error>> {
    let mut key = vec![
        0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04,
        0x20,
    ];
    key.extend_from_slice(&[37; 32]);
    let manifest =
        include_bytes!("../../../../../sdk/plugins/v1/examples/reference-review/manifest.json");
    let icon =
        include_bytes!("../../../../../sdk/plugins/v1/examples/reference-review/assets/review.png");
    let mut declaration: Value = serde_json::from_slice(manifest)?;
    declaration["version"] = json!(version);
    let bytes = sign_package(&serde_json::to_vec(&declaration)?, icon, &key)?;
    Ok((
        PublisherAnchor {
            namespace: "org.example".into(),
            fingerprint: signing_key_fingerprint(&key)?,
        },
        bytes,
    ))
}

#[tokio::test]
async fn review_consumes_only_the_snapshot_selected_and_expires() -> TestResult {
    let profile = tempfile::tempdir()?;
    let admin = DesktopPluginAdmin::default();
    let service = admin.service(profile.path()).await?;
    let (publisher, bytes) = signed_fixture("1.0.0")?;
    service.trust_publisher(publisher).await?;

    let (_, ticket) = admin.review(service, bytes.clone()).await?;
    let ticket = ticket.ok_or("review ticket")?;
    admin.discard(Uuid::new_v4()).await;
    assert!(admin.install(service, Uuid::new_v4()).await.is_err());
    admin
        .pending
        .lock()
        .await
        .as_mut()
        .ok_or("pending review")?
        .expires_at = Instant::now();
    assert!(admin.install(service, ticket).await.is_err());
    let (_, ticket) = admin.review(service, bytes.clone()).await?;
    let ticket = ticket.ok_or("review ticket")?;
    admin.discard(ticket).await;
    assert!(admin.install(service, ticket).await.is_err());
    let selected = profile.path().join("review.mundam-plugin");
    tokio::fs::write(&selected, &bytes).await?;
    let (_, ticket) = admin
        .review(service, read_selected_package(&selected).await?)
        .await?;
    let ticket = ticket.ok_or("review ticket")?;
    tokio::fs::write(&selected, b"replaced after review").await?;
    assert!(!admin.install(service, ticket).await?.enabled);
    assert!(admin.install(service, ticket).await.is_err());
    let plugin_id = "org.example.reference-review";
    let (activation, ..) = admin
        .review_activation(service, "catalog-a".into(), plugin_id)
        .await?;
    assert!(admin
        .enable(service, "catalog-b", activation, vec![])
        .await
        .is_err());
    assert!(admin
        .enable(service, "catalog-a", activation, vec![])
        .await
        .is_err());
    let (activation, ..) = admin
        .review_activation(service, "catalog-a".into(), plugin_id)
        .await?;
    admin
        .enable(service, "catalog-a", activation, vec![])
        .await?;
    assert!(admin
        .enable(service, "catalog-a", activation, vec![])
        .await
        .is_err());
    let oversized = profile.path().join("oversized.mundam-plugin");
    tokio::fs::write(&oversized, vec![0; MAX_PACKAGE_BYTES + 1]).await?;
    assert!(read_selected_package(&oversized).await.is_err());
    Ok(())
}

fn invoke(window: &WebviewWindow<MockRuntime>, command: &str, body: Value) -> Result<Value, Value> {
    tauri::test::get_ipc_response(
        window,
        tauri::webview::InvokeRequest {
            cmd: command.into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: "http://tauri.localhost"
                .parse()
                .map_err(|_| json!("invalid URL"))?,
            body: tauri::ipc::InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.into(),
        },
    )?
    .deserialize()
    .map_err(|error| json!(error.to_string()))
}

#[test]
fn settings_ipc_uses_the_managed_profile_for_trust_and_revocation() -> TestResult {
    let profile = tempfile::tempdir()?;
    let path = profile.path();
    let database = tauri::async_runtime::block_on(DbManager::new(&path.join("catalog.sqlite")))?;
    let app = tauri::test::mock_builder()
        .manage(Arc::new(IpcAdmission::default()))
        .manage(DesktopPluginAdmin::default())
        .manage(Arc::new(database))
        .manage(AppDirectories {
            app_data: path.into(),
            db_path: path.join("catalog.sqlite"),
            settings_path: path.join("settings.json"),
            thumbnails_dir: path.join("thumbnails"),
            previews_dir: path.join("previews"),
        })
        .invoke_handler(tauri::generate_handler![
            list_plugin_settings,
            get_plugin_presentation,
            trust_plugin_publisher,
            revoke_plugin_publisher,
            install_reviewed_plugin,
            review_plugin_activation,
            enable_reviewed_plugin,
            disable_installed_plugin,
            review_plugin_rollback,
            remove_installed_plugin
        ])
        .build(crate::application_context::<MockRuntime>())?;
    let window = WebviewWindowBuilder::new(&app, "main", Default::default()).build()?;
    let auxiliary = WebviewWindowBuilder::new(&app, "design-system", Default::default()).build()?;
    assert!(invoke(&auxiliary, "list_plugin_settings", json!({})).is_err());
    assert!(invoke(&auxiliary, "get_plugin_presentation", json!({})).is_err());
    assert!(!path.join("plugins.sqlite").exists());
    let call =
        |command, payload| invoke(&window, command, payload).map_err(|error| error.to_string());
    assert_eq!(
        call("list_plugin_settings", json!({}))?,
        json!({"publishers": [], "installed": [], "safeMode": false})
    );
    assert!(path.join("plugins.sqlite").is_file());
    let identity = json!({"namespace": "org.other", "fingerprint": "AB".repeat(32)});
    call("trust_plugin_publisher", identity.clone())?;
    let publishers = call("list_plugin_settings", json!({}))?;
    assert_eq!(publishers["publishers"][0]["fingerprint"], "ab".repeat(32));
    assert_eq!(publishers["publishers"][0]["revoked"], false);
    call("revoke_plugin_publisher", identity)?;
    assert_eq!(
        call("list_plugin_settings", json!({}))?["publishers"][0]["revoked"],
        true
    );
    assert!(call(
        "install_reviewed_plugin",
        json!({"reviewId": Uuid::new_v4()})
    )
    .is_err());
    let admin = app.state::<DesktopPluginAdmin>();
    let (publisher, bytes) = signed_fixture("1.0.0")?;
    tauri::async_runtime::block_on(async {
        let service = admin.service(path).await?;
        service.trust_publisher(publisher).await?;
        let (_, ticket) = admin.review(service, bytes).await?;
        admin
            .install(service, ticket.ok_or_else(expired_review)?)
            .await
    })?;
    let reference = json!({"pluginId": "org.example.reference-review"});
    let activation = call("review_plugin_activation", reference.clone())?;
    assert_eq!(
        activation["requiredPermissions"],
        json!([
            "selection.read",
            "assetData.own.read",
            "assetData.own.write"
        ])
    );
    call(
        "enable_reviewed_plugin",
        json!({"reviewId": activation["reviewId"], "optionalPermissions": []}),
    )?;
    assert_eq!(
        call("list_plugin_settings", json!({}))?["installed"][0]["enabled"],
        true
    );
    let presentation = call("get_plugin_presentation", json!({}))?;
    assert_eq!(
        presentation["sections"][0]["fields"][0],
        json!({
            "fieldId": "reviewed", "kind": "checkbox", "label": "Reference reviewed", "defaultValue": false
        })
    );
    call("disable_installed_plugin", reference.clone())?;
    assert_eq!(
        call("get_plugin_presentation", json!({}))?,
        json!({"sections": [], "screens": [], "commands": []})
    );
    assert_eq!(
        call("list_plugin_settings", json!({}))?["installed"][0]["approved"],
        false
    );
    let (_, update) = signed_fixture("1.1.0")?;
    let reviewed = tauri::async_runtime::block_on(async {
        let service = admin.service(path).await?;
        let (_, ticket) = admin.review(service, update).await?;
        ticket.ok_or_else(expired_review)
    })?;
    let installed = call("install_reviewed_plugin", json!({"reviewId": reviewed}))?;
    assert_eq!(installed["rollbackVersion"], "1.0.0");
    let rollback = call("review_plugin_rollback", reference)?;
    assert_eq!(rollback["replacement"]["kind"], "rollback");
    assert_eq!(rollback["replacement"]["currentVersion"], "1.1.0");
    let restored = call(
        "install_reviewed_plugin",
        json!({"reviewId": rollback["reviewId"]}),
    )?;
    assert_eq!(restored["version"], "1.0.0");
    call(
        "remove_installed_plugin",
        json!({
            "pluginId": restored["id"],
            "packageRevision": restored["packageRevision"]
        }),
    )?;
    assert_eq!(
        call("list_plugin_settings", json!({}))?["installed"],
        json!([])
    );
    Ok(())
}
