use super::*;
use crate::core::error::AppResult;
use serde_json::{json, Value};
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tauri::{Manager, State};
use tauri_plugin_fs::FsExt;
use tokio::sync::Notify;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn import_preferences_use_managed_catalog_through_ipc() -> TestResult {
    use crate::infra::database::manager::DbManager;

    let directory = tempfile::tempdir()?;
    let database = tauri::async_runtime::block_on(async {
        let database = DbManager::new(&directory.path().join("catalog.sqlite")).await?;
        sqlx::query("INSERT INTO folders (id, name, path) VALUES ('photos', 'Photos', '/photos')")
            .execute(database.pool())
            .await?;
        Ok::<_, AppError>(Arc::new(database))
    })?;
    let app = tauri::test::mock_builder()
        .manage(Arc::new(IpcAdmission::default()))
        .manage(database.clone())
        .invoke_handler(tauri::generate_handler![
            crate::delivery::tauri::commands::library::get_import_preference,
            crate::delivery::tauri::commands::library::set_import_preference
        ])
        .build(crate::application_context::<tauri::test::MockRuntime>())?;
    let window = tauri::WebviewWindowBuilder::new(&app, "main", Default::default()).build()?;
    let call = |command, body| invoke(&window, command, body).map_err(|error| error.to_string());
    let initial = call("get_import_preference", json!({}))?;
    assert!(initial["folderId"].is_null());
    let catalog_id = initial["catalogId"].as_str().ok_or("Missing catalog ID")?;
    let saved = call(
        "set_import_preference",
        json!({"catalogId": catalog_id, "folderId": "photos"}),
    )?;
    assert_eq!(saved["folderId"], "photos");
    assert_eq!(call("get_import_preference", json!({}))?, saved);
    assert_eq!(
        call(
            "set_import_preference",
            json!({"catalogId": catalog_id, "folderId": null})
        )?,
        initial
    );
    tauri::async_runtime::block_on(database.close());
    Ok(())
}

fn invoke(
    window: &tauri::WebviewWindow<tauri::test::MockRuntime>,
    command: &str,
    body: Value,
) -> Result<Value, Value> {
    tauri::test::get_ipc_response(
        window,
        tauri::webview::InvokeRequest {
            cmd: command.into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: "http://tauri.localhost"
                .parse()
                .map_err(|error| json!(format!("{error}")))?,
            body: tauri::ipc::InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.into(),
        },
    )?
    .deserialize()
    .map_err(|error| json!(error.to_string()))
}

#[tokio::test]
async fn close_waits_for_existing_scopes_and_rejects_late_entries() -> TestResult {
    let admission = Arc::new(IpcAdmission::default());
    let first = admission.enter("read")?;
    let second = admission.enter("write")?;
    assert_eq!(admission.close(), 2);
    assert!(matches!(admission.enter("late"), Err(AppError::Cancelled)));
    assert!(
        tokio::time::timeout(Duration::from_millis(20), admission.drain())
            .await
            .is_err()
    );
    drop(first);
    assert!(!admission.is_drained());
    drop(second);
    tokio::time::timeout(Duration::from_secs(2), admission.drain()).await?;
    assert!(admission.is_drained());
    assert_eq!(admission.close(), 0);
    Ok(())
}

#[tokio::test]
async fn rejected_lifecycle_owner_cannot_admit_commands() -> TestResult {
    let lifecycle = Arc::new(LifecycleRegistry::new());
    assert!(lifecycle.shutdown_all().await);
    let admission = Arc::new(IpcAdmission::default());
    assert!(!admission.register_lifecycle_owner(&lifecycle));
    assert!(admission.enter("late").is_err());
    Ok(())
}

#[test]
fn real_command_extraction_rejects_all_return_types_before_missing_state_or_arguments() -> TestResult
{
    let admission = Arc::new(IpcAdmission::default());
    let app = tauri::test::mock_builder()
        .manage(admission.clone())
        .manage(crate::feature::startup::StartupService::default())
        .manage(crate::delivery::tauri::commands::queries::StreamingSessionToken("fixture".into()))
        .invoke_handler(tauri::generate_handler![
            crate::delivery::tauri::commands::queries::get_streaming_token,
            crate::delivery::tauri::commands::queries::get_asset,
            crate::delivery::tauri::commands::streaming::get_quality_options,
            crate::delivery::tauri::commands::streaming::ffmpeg_available,
            crate::delivery::tauri::startup::get_startup_state,
        ])
        .build(crate::application_context::<tauri::test::MockRuntime>())?;
    let window = tauri::WebviewWindowBuilder::new(&app, "main", Default::default()).build()?;
    assert_eq!(
        invoke(&window, "get_streaming_token", json!({})).map_err(|error| error.to_string())?,
        "fixture"
    );
    // Decoding fails after extracting the guard; it must be released even then.
    assert!(invoke(&window, "get_asset", json!({})).is_err());
    assert!(admission.active_commands().is_empty());
    admission.close();
    for command in [
        "get_streaming_token",
        "get_asset",
        "get_quality_options",
        "ffmpeg_available",
    ] {
        let error = invoke(&window, command, json!({"_ipc": "cannot bypass admission"}))
            .err()
            .ok_or("late command accepted")?;
        assert_eq!(error["code"], "CANCELLED", "{command}: {error}");
    }
    assert!(invoke(&window, "get_startup_state", json!({})).is_ok());
    assert!(admission.is_drained());
    Ok(())
}

struct TransactionBarrier {
    entered: Notify,
    release: Notify,
}

// Uses an existing production permission, with a deterministic transaction for the gate test.
#[tauri::command]
async fn run_db_maintenance(
    _ipc: IpcRequest,
    pool: State<'_, sqlx::SqlitePool>,
    barrier: State<'_, Arc<TransactionBarrier>>,
) -> AppResult<()> {
    let mut transaction = pool.begin().await?;
    sqlx::query("INSERT INTO r3_fixture VALUES (1)")
        .execute(&mut *transaction)
        .await?;
    barrier.entered.notify_one();
    barrier.release.notified().await;
    transaction.commit().await?;
    Ok(())
}

#[tokio::test]
async fn admitted_ipc_transaction_keeps_bridge_and_pool_open_until_commit() -> TestResult {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;
    sqlx::query("CREATE TABLE r3_fixture (value INTEGER)")
        .execute(&pool)
        .await?;
    let admission = Arc::new(IpcAdmission::default());
    let lifecycle = Arc::new(LifecycleRegistry::new());
    assert!(admission.register_lifecycle_owner(&lifecycle));
    let bridge_stop = tokio_util::sync::CancellationToken::new();
    let bridge_stopped = Arc::new(AtomicBool::new(false));
    let stopped = bridge_stopped.clone();
    let bridge_token = bridge_stop.clone();
    lifecycle.register(
        "event_bridge_frontend".into(),
        bridge_stop,
        tauri::async_runtime::spawn(async move {
            bridge_token.cancelled().await;
            stopped.store(true, Ordering::Release);
        }),
    );
    let barrier = Arc::new(TransactionBarrier {
        entered: Notify::new(),
        release: Notify::new(),
    });
    let app = tauri::test::mock_builder()
        .manage(admission.clone())
        .manage(pool.clone())
        .manage(barrier.clone())
        .invoke_handler(tauri::generate_handler![run_db_maintenance])
        .build(crate::application_context::<tauri::test::MockRuntime>())?;
    let window = tauri::WebviewWindowBuilder::new(&app, "main", Default::default()).build()?;
    let caller_window = window.clone();
    let caller = tokio::task::spawn_blocking(move || {
        invoke(&caller_window, "run_db_maintenance", json!({}))
    });
    tokio::time::timeout(Duration::from_secs(3), barrier.entered.notified()).await?;
    admission.close();
    let registry = lifecycle.clone();
    let mut shutdown = tokio::spawn(async move { registry.shutdown_all().await });
    assert!(
        tokio::time::timeout(Duration::from_millis(30), &mut shutdown)
            .await
            .is_err()
    );
    assert!(!pool.is_closed());
    assert!(!bridge_stopped.load(Ordering::Acquire));
    assert!(!admission.is_drained());
    barrier.release.notify_one();
    caller.await?.map_err(|error| error.to_string())?;
    assert!(tokio::time::timeout(Duration::from_secs(3), shutdown).await??);
    assert!(admission.is_drained());
    assert!(bridge_stopped.load(Ordering::Acquire));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM r3_fixture")
            .fetch_one(&pool)
            .await?,
        1
    );
    pool.close().await;
    assert!(pool.is_closed());
    let error = invoke(&window, "run_db_maintenance", json!({}))
        .err()
        .ok_or("query admitted after pool closed")?;
    assert_eq!(error["code"], "CANCELLED");
    Ok(())
}

#[test]
fn text_transfers_preserve_scope_and_cannot_write_after_close() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("theme.json");
    std::fs::write(&path, "original")?;
    let admission = Arc::new(IpcAdmission::default());
    let app = tauri::test::mock_builder()
        .manage(admission.clone())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            crate::delivery::tauri::commands::text_files::read_text_file,
            crate::delivery::tauri::commands::text_files::write_text_file,
        ])
        .build(crate::application_context::<tauri::test::MockRuntime>())?;
    let window = tauri::WebviewWindowBuilder::new(&app, "main", Default::default()).build()?;
    let denied = invoke(&window, "read_text_file", json!({"path": path}))
        .err()
        .ok_or("unauthorized read accepted")?;
    assert_ne!(denied["code"], "CANCELLED");
    app.fs_scope().allow_file(&path)?;
    assert_eq!(
        invoke(&window, "read_text_file", json!({"path": path}))
            .map_err(|error| error.to_string())?,
        "original"
    );
    invoke(
        &window,
        "write_text_file",
        json!({"path": path, "contents": "exported"}),
    )
    .map_err(|error| error.to_string())?;
    assert_eq!(std::fs::read_to_string(&path)?, "exported");
    assert!(invoke(
        &window,
        "plugin:fs|write_text_file",
        json!({"path": path, "contents": "bypass"})
    )
    .is_err());
    let forbidden = directory.path().join("private.json");
    std::fs::write(&forbidden, "private")?;
    app.fs_scope().allow_file(&forbidden)?;
    app.add_capability(
        tauri::ipc::CapabilityBuilder::new("r3-denied-text")
            .window("main")
            .permission_scoped(
                "fs:scope",
                Vec::<Value>::new(),
                vec![json!({"path": forbidden})],
            ),
    )?;
    for command in ["read_text_file", "write_text_file"] {
        let error = invoke(
            &window,
            command,
            json!({"path": forbidden, "contents": "bypass"}),
        )
        .err()
        .ok_or("global filesystem denial bypassed by a dynamic grant")?;
        assert_eq!(error["code"], "INTERNAL_ERROR");
        assert!(error["message"]
            .as_str()
            .is_some_and(|message| message.contains("authorized filesystem scope")));
    }
    assert_eq!(std::fs::read_to_string(forbidden)?, "private");
    admission.close();
    let error = invoke(
        &window,
        "write_text_file",
        json!({"path": path, "contents": "late"}),
    )
    .err()
    .ok_or("late export accepted")?;
    assert_eq!(error["code"], "CANCELLED");
    assert_eq!(std::fs::read_to_string(path)?, "exported");
    Ok(())
}

#[test]
fn every_registered_business_command_declares_admission() -> TestResult {
    let sources = [
        include_str!("commands/queries.rs"),
        include_str!("commands/mutations.rs"),
        include_str!("commands/settings.rs"),
        include_str!("commands/external_import.rs"),
        include_str!("commands/external_import_pairing.rs"),
        include_str!("commands/duplicates.rs"),
        include_str!("commands/streaming.rs"),
        include_str!("commands/library.rs"),
        include_str!("commands/text_files.rs"),
        include_str!("processing.rs"),
        include_str!("thumbnails.rs"),
    ];
    let mut guarded = std::collections::BTreeSet::new();
    for source in sources {
        for command in source.split("#[tauri::command]").skip(1) {
            let signature = command
                .split('{')
                .next()
                .ok_or("missing command signature")?;
            let name = signature
                .split("fn ")
                .nth(1)
                .ok_or("missing function")?
                .split(['(', '<'])
                .next()
                .ok_or("missing command name")?
                .trim();
            if name != "send_telemetry_log" {
                assert!(
                    signature.contains("_ipc: IpcRequest"),
                    "unguarded command: {name}"
                );
                guarded.insert(name.to_owned());
            }
        }
    }
    let controls = [
        "get_startup_state",
        "report_startup_stage",
        "complete_startup",
        "fail_frontend_startup",
        "get_shutdown_state",
        "acknowledge_shutdown_presentation",
        "send_telemetry_log",
    ];
    let registered = include_str!("../../lib.rs")
        .split("tauri::generate_handler![")
        .nth(1)
        .ok_or("missing handler")?
        .split("])")
        .next()
        .ok_or("missing handler end")?;
    for line in registered
        .lines()
        .filter(|line| line.trim_start().starts_with("delivery::tauri::"))
    {
        let name = line
            .trim()
            .trim_end_matches(',')
            .rsplit("::")
            .next()
            .ok_or("missing registration")?;
        assert!(
            guarded.contains(name) || controls.contains(&name),
            "unclassified IPC registration: {name}"
        );
    }
    assert!(guarded.len() >= 84);
    Ok(())
}
