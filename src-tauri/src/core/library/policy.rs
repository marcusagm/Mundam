//! Pure validation for user intentions, never for observing existing external files.
use crate::core::error::{AppError, AppResult};

/// Validates a portable new entry name without accepting traversal components.
///
/// # Errors
/// Rejects empty names, separators, Windows reserved stems and trailing dots/spaces.
pub fn validate_name(name: &str) -> AppResult<()> {
    let stem = name
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    let reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    if name.trim().is_empty()
        || name == "."
        || name == ".."
        || name.ends_with([' ', '.'])
        || name
            .chars()
            .any(|character| character.is_control() || "<>:\"/\\|?*".contains(character))
        || reserved.contains(&stem.as_str())
    {
        return Err(AppError::ValidationFailed(
            "Use a valid entry name without path separators".into(),
        ));
    }
    Ok(())
}

/// Default logical recovery capacity; archives never expire automatically.
pub const DEFAULT_RECOVERY_BUDGET: i64 = 50 * 1024 * 1024 * 1024;

/// Bounds individual-file plans; whole-folder moves use one native subtree movement.
pub const MAX_MOVE_SELECTION: usize = 5_000;

/// Resolves the trusted settings preference without accepting quota overrides from IPC intentions.
///
/// # Errors
/// Rejects nonnumeric, nonpositive or out-of-range persisted settings.
pub fn recovery_budget(value: Option<&serde_json::Value>) -> AppResult<i64> {
    let Some(value) = value else {
        return Ok(DEFAULT_RECOVERY_BUDGET);
    };
    value
        .as_i64()
        .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
        .filter(|value| *value > 0)
        .ok_or_else(|| {
            AppError::ValidationFailed("Recovery storage budget must be a positive integer".into())
        })
}
