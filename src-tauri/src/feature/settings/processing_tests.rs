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
        Ok(self.settings.lock().unwrap().clone())
    }
    async fn save(&self, settings: &AppSettings) -> AppResult<()> {
        tokio::task::yield_now().await;
        if self.fail.load(Ordering::SeqCst) {
            return Err(AppError::Internal("disk unavailable".into()));
        }
        *self.settings.lock().unwrap() = settings.clone();
        Ok(())
    }
}

#[tokio::test]
async fn processing_settings_apply_only_after_successful_persistence() {
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
        .await
        .unwrap();
    service
        .set_setting("thumbnail_threads".into(), "1".into())
        .await
        .unwrap();
    assert_eq!(control.budget.snapshot().limit, 2);
    assert_eq!(control.budget.snapshot().thumbnail_limit, 1);
    assert_eq!(repository.load().await.unwrap().processing_concurrency, 2);
}

#[tokio::test]
async fn processing_settings_serialize_cloned_service_writes_without_lost_updates() {
    let repository = Arc::new(Repository::default());
    let service = SettingsService::new(repository.clone());
    let cloned = service.clone();
    let (shared, thumbnail) = tokio::join!(
        service.set_setting("processing_concurrency".into(), "2".into()),
        cloned.set_setting("thumbnail_threads".into(), "1".into())
    );
    shared.unwrap();
    thumbnail.unwrap();
    let settings = repository.load().await.unwrap();
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
    assert_eq!(repository.load().await.unwrap().processing_concurrency, 2);
}

#[test]
fn processing_settings_load_legacy_configuration_without_shared_budget() {
    let mut legacy = serde_json::to_value(AppSettings::default()).unwrap();
    legacy
        .as_object_mut()
        .unwrap()
        .remove("processing_concurrency");
    let settings: AppSettings = serde_json::from_value(legacy).unwrap();
    assert_eq!(settings.processing_concurrency, 0);
}
