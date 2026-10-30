//! Exercises real command authorization using production capabilities and a mock desktop runtime.

use super::get_thumbnail_processing_status;
use crate::core::processing::{ProcessingKind, ProcessingOutcome, ProcessingRepository};
use crate::processing::workers::test_support::{ProcessingFixture, TestResult};
use serde_json::{json, Value};
use std::sync::Arc;

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
    let app = tauri::test::mock_builder()
        .manage(repository)
        .invoke_handler(tauri::generate_handler![get_thumbnail_processing_status])
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
    }
    Ok(())
}
