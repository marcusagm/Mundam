//! Exercises real command authorization using production capabilities and a mock desktop runtime.

use super::get_thumbnail_processing_status;
use crate::core::processing::control::{ProcessingControl, WorkerKind};
use crate::core::processing::{ProcessingKind, ProcessingOutcome, ProcessingRepository};
use crate::processing::workers::test_support::{ProcessingFixture, TestResult};
use serde_json::{json, Value};
use std::sync::Arc;

struct AvailableTools;
#[async_trait::async_trait]
impl crate::core::processing::tools::ExtractionToolProbe for AvailableTools {
    async fn available(
        &self,
        _: crate::core::processing::tools::ExtractionTool,
    ) -> crate::core::AppResult<bool> {
        Ok(true)
    }
}

fn invoke_processing(
    window: &tauri::WebviewWindow<tauri::test::MockRuntime>,
    command: &str,
    body: Value,
) -> Result<Value, String> {
    tauri::test::get_ipc_response(
        window,
        tauri::webview::InvokeRequest {
            cmd: command.into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: "http://tauri.localhost"
                .parse()
                .map_err(|error| format!("{error}"))?,
            body: tauri::ipc::InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.into(),
        },
    )
    .map_err(|error| format!("{error:?}"))?
    .deserialize::<Value>()
    .map_err(|error| error.to_string())
}

#[test]
fn processing_thumbnail_status_ipc_authorizes_library_windows_and_rejects_splashscreen(
) -> TestResult {
    let fixture = tauri::async_runtime::block_on(async {
        let fixture = ProcessingFixture::new().await?;
        fixture.asset("failed-heic", None).await?;
        fixture.asset("queued", None).await?;
        for _ in 0..3 {
            let ticket = fixture
                .processing
                .begin(ProcessingKind::Thumbnail, "failed-heic")
                .await?
                .ok_or("Missing retry")?;
            fixture
                .processing
                .finish(
                    &ticket,
                    ProcessingOutcome::Failed("FFmpeg extraction failed".into()),
                )
                .await?;
            sqlx::query(
                "UPDATE processing_tasks SET next_attempt_at = 0 WHERE asset_id = 'failed-heic'",
            )
            .execute(&fixture.pool)
            .await?;
        }
        Ok::<_, Box<dyn std::error::Error>>(fixture)
    })?;
    let repository: Arc<dyn ProcessingRepository> = fixture.processing.clone();
    let control = Arc::new(ProcessingControl::default());
    let duplicates = crate::feature::duplicates::commands::DuplicateCommandService::new(
        Arc::new(
            crate::infra::database::duplicates::SqliteDuplicatesRepository::new(
                fixture.pool.clone(),
            ),
        ),
        fixture.ledger.clone(),
        fixture.event_bus.clone(),
        fixture.queries.clone(),
        fixture.registry.clone(),
        fixture.processing.clone(),
    )
    .with_processing_control(control.clone());
    let settings = Arc::new(
        crate::infra::config::json_adapter::JsonSettingsAdapter::new(
            fixture.data.path().join("settings.json"),
        ),
    );
    let service = Arc::new(
        crate::feature::processing::control::ProcessingControlService::new(
            Arc::new(
                crate::infra::database::duplicates::SqliteDuplicatesRepository::new(
                    fixture.pool.clone(),
                ),
            ),
            control.clone(),
            repository.clone(),
            settings,
            Arc::new(duplicates),
        )
        .with_tools(Arc::new(
            crate::feature::processing::tools::ProcessingToolService::new(
                repository.clone(),
                Arc::new(AvailableTools),
            ),
        )),
    );
    let app = tauri::test::mock_builder()
        .manage(repository)
        .manage(service)
        .invoke_handler(tauri::generate_handler![
            get_thumbnail_processing_status,
            crate::delivery::tauri::processing::get_processing_snapshot,
            crate::delivery::tauri::processing::set_processing_worker_paused,
            crate::delivery::tauri::processing::retry_processing_worker,
            crate::delivery::tauri::processing::check_processing_tools
        ])
        .build(tauri::generate_context!())?;
    for label in ["main", "design-system", "splashscreen"] {
        let window = tauri::WebviewWindowBuilder::new(&app, label, Default::default()).build()?;
        let response = tauri::test::get_ipc_response(
            &window,
            tauri::webview::InvokeRequest {
                cmd: "get_thumbnail_processing_status".into(),
                callback: tauri::ipc::CallbackFn(0),
                error: tauri::ipc::CallbackFn(1),
                url: "http://tauri.localhost".parse()?,
                body: tauri::ipc::InvokeBody::Json(
                    json!({ "assetIds": ["failed-heic", "queued"] }),
                ),
                headers: Default::default(),
                invoke_key: tauri::test::INVOKE_KEY.into(),
            },
        );
        if label == "splashscreen" {
            assert!(invoke_processing(&window, "check_processing_tools", json!({})).is_err());
            assert!(invoke_processing(&window, "get_processing_snapshot", json!({})).is_err());
            assert!(invoke_processing(
                &window,
                "set_processing_worker_paused",
                json!({"worker":"thumbnails", "paused":true})
            )
            .is_err());
            assert!(invoke_processing(
                &window,
                "retry_processing_worker",
                json!({"worker":"colors", "scope":"failed"})
            )
            .is_err());
            assert!(
                response.is_err(),
                "Thumbnail statuses must not be exposed to the splashscreen"
            );
            continue;
        }
        let statuses = response
            .map_err(|error| format!("{label} IPC rejected: {error:?}"))?
            .deserialize::<Vec<Value>>()?;
        assert_eq!(statuses.len(), 2);
        for (identifier, state) in [("failed-heic", "failed"), ("queued", "pending")] {
            let status = statuses
                .iter()
                .find(|status| status["asset_id"] == identifier)
                .ok_or("Missing asset status")?;
            assert_eq!(status["state"], state);
            assert!(status["thumbnail_path"].is_null());
        }
        let checked = invoke_processing(&window, "check_processing_tools", json!({}))?;
        assert_eq!(checked["tools"].as_array().ok_or("Missing tools")?.len(), 3);
        assert_eq!(checked["tools"][0]["state"], "available");
        let initial = invoke_processing(&window, "get_processing_snapshot", json!({}))?;
        assert_eq!(
            initial["workers"]
                .as_array()
                .ok_or("Missing workers")?
                .len(),
            3
        );
        for kind in WorkerKind::ALL {
            let worker = serde_json::to_value(kind)?;
            let activity = control.try_admit(kind, "test_operation", None);
            let paused = invoke_processing(
                &window,
                "set_processing_worker_paused",
                json!({"worker":worker,"paused":true}),
            )?;
            let row = paused["workers"]
                .as_array()
                .ok_or("Missing workers")?
                .iter()
                .find(|row| row["worker"] == worker)
                .ok_or("Missing row")?;
            assert_eq!(row["paused"], true);
            if kind == WorkerKind::Colors {
                let retry = invoke_processing(
                    &window,
                    "retry_processing_worker",
                    json!({"worker":"colors", "scope":"failed"}),
                )?;
                assert_eq!(retry["assets"], 0);
                assert_eq!(retry["comparisons"], 0);
                assert_eq!(retry["snapshot"]["workers"][1]["paused"], true);
                assert!(invoke_processing(
                    &window,
                    "retry_processing_worker",
                    json!({"worker":"colors", "scope":"all"})
                )
                .is_err());
            }

            assert_eq!(row["active"], 1);
            assert!(control.try_admit(kind, "blocked", None).is_none());
            drop(activity);
            let resumed = invoke_processing(
                &window,
                "set_processing_worker_paused",
                json!({"worker":worker,"paused":false}),
            )?;
            assert!(resumed["sequence"].as_u64() > paused["sequence"].as_u64());
            assert!(control.try_admit(kind, "resumed", None).is_some());
        }
        assert!(invoke_processing(
            &window,
            "set_processing_worker_paused",
            json!({"worker":"unknown", "paused":true})
        )
        .is_err());
    }
    Ok(())
}
