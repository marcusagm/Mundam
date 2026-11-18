//! Opt-in video sink compatibility configured before WebKit and worker startup.

use crate::core::{AppError, AppResult};
use std::path::Path;

const SETTING: &str = "linux_video_compatibility";
const SINK_OVERRIDE: &str = "WEBKIT_GST_DISABLE_GL_SINK";

/// Applies the saved Linux video policy before creating any WebView.
///
/// Explicit process environment overrides are preserved for diagnostics. The
/// normal path is unchanged unless the user saved compatibility mode.
/// Must run on the initial thread, before GTK and application workers start.
pub fn apply(identifier: &str) {
    if std::env::var_os(SINK_OVERRIDE).is_some() {
        tracing::info!(
            source = "environment",
            "Linux video sink override preserved"
        );
        return;
    }
    let Some(data_directory) = dirs::data_local_dir() else {
        tracing::warn!("Linux video settings directory unavailable; keeping normal video mode");
        return;
    };
    let settings_path = data_directory.join(identifier).join("settings.json");
    match read_saved_mode(&settings_path) {
        Ok(enabled) => {
            if enabled {
                // This runs before Tauri/GTK and all application background tasks.
                std::env::set_var(SINK_OVERRIDE, "1");
            }
            tracing::info!(
                compatibility_enabled = enabled,
                source = "settings",
                "Linux video mode selected"
            );
        }
        Err(error) => {
            tracing::warn!(%error, "Unable to read Linux video mode; keeping normal video mode")
        }
    }
}

/// Reads only the early video preference without creating or rewriting settings.
///
/// # Errors
/// Reports unreadable JSON or a non-boolean persisted compatibility value.
fn read_saved_mode(path: &Path) -> AppResult<bool> {
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(AppError::Internal(format!(
                "Unable to read video settings: {error}"
            )))
        }
    };
    let settings: serde_json::Value = serde_json::from_str(&contents)
        .map_err(|error| AppError::Internal(format!("Unable to parse video settings: {error}")))?;
    match settings.get("extra").and_then(|extra| extra.get(SETTING)) {
        None => Ok(false),
        Some(value) => value.as_bool().ok_or_else(|| {
            AppError::ValidationFailed("Linux video compatibility must be a boolean".into())
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feature::settings::SettingsService;
    use crate::infra::config::json_adapter::JsonSettingsAdapter;
    use std::sync::Arc;

    #[test]
    fn linux_video_defaults_to_normal_for_missing_or_older_settings() -> AppResult<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("settings.json");
        assert!(!read_saved_mode(&path)?);
        std::fs::write(&path, r#"{"extra":{"theme":"dark"}}"#)?;
        assert!(!read_saved_mode(&path)?);
        Ok(())
    }

    #[tokio::test]
    async fn linux_video_saved_mode_is_consumed_by_early_startup() -> AppResult<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("settings.json");
        let service = SettingsService::new(Arc::new(JsonSettingsAdapter::new(path.clone())));
        service.set_setting(SETTING.into(), "true".into()).await?;
        assert!(read_saved_mode(&path)?);
        assert_eq!(
            service.get_setting(SETTING).await?,
            Some(serde_json::Value::Bool(true))
        );
        assert!(service
            .set_setting(SETTING.into(), "invalid".into())
            .await
            .is_err());
        assert!(read_saved_mode(&path)?);
        service.set_setting(SETTING.into(), "false".into()).await?;
        assert!(!read_saved_mode(&path)?);
        let mut settings = service.get_settings().await?;
        settings.extra.insert(SETTING.into(), "invalid".into());
        assert!(service.update_settings(settings).await.is_err());
        assert!(!read_saved_mode(&path)?);
        Ok(())
    }

    #[test]
    fn linux_video_startup_environment_child() -> AppResult<()> {
        if std::env::var_os("MUNDAM_VIDEO_MODE_TEST_CHILD").is_none() {
            return Ok(());
        }
        apply("test.video.mode");
        let actual = std::env::var(SINK_OVERRIDE).unwrap_or_default();
        let expected = std::env::var("MUNDAM_VIDEO_MODE_EXPECTED").unwrap_or_default();
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn linux_video_applies_only_opt_in_and_preserves_environment() -> AppResult<()> {
        let directory = tempfile::tempdir()?;
        let settings_directory = directory.path().join("test.video.mode");
        std::fs::create_dir(&settings_directory)?;
        for (enabled, external, expected) in [
            (false, None, ""),
            (true, None, "1"),
            (true, Some("0"), "0"),
            (false, Some("1"), "1"),
        ] {
            std::fs::write(
                settings_directory.join("settings.json"),
                serde_json::json!({"extra": {"linux_video_compatibility": enabled}}).to_string(),
            )?;
            let mut command = std::process::Command::new(std::env::current_exe()?);
            command
                .args([
                    "--exact",
                    "bootstrap::linux_video::tests::linux_video_startup_environment_child",
                ])
                .env("XDG_DATA_HOME", directory.path())
                .env("MUNDAM_VIDEO_MODE_TEST_CHILD", "1")
                .env("MUNDAM_VIDEO_MODE_EXPECTED", expected)
                .env_remove(SINK_OVERRIDE);
            if let Some(value) = external {
                command.env(SINK_OVERRIDE, value);
            }
            let output = command.output()?;
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stdout)
            );
        }
        Ok(())
    }

    #[test]
    fn linux_video_rejects_invalid_or_incomplete_preferences() -> AppResult<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("settings.json");
        for contents in [r#"{"extra":{"linux_video_compatibility":"true"}}"#, "{"] {
            std::fs::write(&path, contents)?;
            assert!(read_saved_mode(&path).is_err());
        }
        Ok(())
    }
}
