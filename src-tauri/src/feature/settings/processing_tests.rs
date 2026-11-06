use super::service::SettingsService;
use crate::core::{
    processing::control::ProcessingControl,
    settings::{AppSettings, SettingsRepository},
    AppError, AppResult,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

#[derive(Default)]
struct Repository {
    settings: Mutex<AppSettings>,
    fail: AtomicBool,
}
#[async_trait::async_trait]
impl SettingsRepository for Repository {
    async fn load(&self) -> AppResult<AppSettings> {
        Ok(self
            .settings
            .lock()
            .map_err(|_| AppError::Internal("Test settings lock poisoned".into()))?
            .clone())
    }
    async fn save(&self, settings: &AppSettings) -> AppResult<()> {
        tokio::task::yield_now().await;
        if self.fail.load(Ordering::SeqCst) {
            return Err(AppError::Internal("disk unavailable".into()));
        }
        *self
            .settings
            .lock()
            .map_err(|_| AppError::Internal("Test settings lock poisoned".into()))? =
            settings.clone();
        Ok(())
    }
}

#[tokio::test]
async fn processing_settings_apply_only_after_successful_persistence(
) -> Result<(), Box<dyn std::error::Error>> {
    let repository = Arc::new(Repository::default());
    let control = Arc::new(ProcessingControl::default());
    control.budget.configure(4, 4);
    let service = SettingsService::new(repository.clone()).with_control(control.clone());
    repository.fail.store(true, Ordering::SeqCst);
    assert!(service
        .set_setting("processing_concurrency".into(), "1".into())
        .await
        .is_err());
    assert_eq!(control.budget.snapshot().limit, 4);
    repository.fail.store(false, Ordering::SeqCst);
    service
        .set_setting("processing_concurrency".into(), "2".into())
        .await?;
    service
        .set_setting("thumbnail_threads".into(), "1".into())
        .await?;
    assert_eq!(control.budget.snapshot().limit, 2);
    assert_eq!(control.budget.snapshot().thumbnail_limit, 1);
    assert_eq!(repository.load().await?.processing_concurrency, 2);

    Ok(())
}

#[tokio::test]
async fn processing_settings_serialize_cloned_service_writes_without_lost_updates(
) -> Result<(), Box<dyn std::error::Error>> {
    let repository = Arc::new(Repository::default());
    let service = SettingsService::new(repository.clone());
    let cloned = service.clone();
    let (shared, thumbnail) = tokio::join!(
        service.set_setting("processing_concurrency".into(), "2".into()),
        cloned.set_setting("thumbnail_threads".into(), "1".into())
    );
    shared?;
    thumbnail?;
    let settings = repository.load().await?;
    assert_eq!(settings.processing_concurrency, 2);
    assert_eq!(settings.thumbnail_threads, 1);
    for value in [
        serde_json::json!(-1),
        serde_json::json!(9),
        serde_json::json!(1.5),
        serde_json::json!("invalid"),
    ] {
        assert!(service
            .set_setting("processing_concurrency".into(), value)
            .await
            .is_err());
    }
    assert_eq!(repository.load().await?.processing_concurrency, 2);

    Ok(())
}

#[test]
fn processing_settings_load_legacy_configuration_without_shared_budget(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut legacy = serde_json::to_value(AppSettings::default())?;
    legacy
        .as_object_mut()
        .ok_or("Expected a value in processing_settings_load_legacy_configuration_without_shared_budget")?
        .remove("processing_concurrency");
    let settings: AppSettings = serde_json::from_value(legacy)?;
    assert_eq!(settings.processing_concurrency, 0);

    Ok(())
}
