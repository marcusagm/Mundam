use thiserror::Error;

/// The primary Result type for the application domain.
pub type AppResult<T> = Result<T, AppError>;

/// Centralized error enum for the application.
///
/// This enum captures errors from infrastructure and domain logic,
/// and maps them to high-level domain errors.
#[derive(Debug, Error)]
pub enum AppError {
    /// Error related to database operations.
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Error related to database migrations.
    #[error("Migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    /// Error related to Tauri framework operations.
    #[error("Tauri error: {0}")]
    Tauri(#[from] tauri::Error),

    /// Error related to filesystem watchers.
    #[error("Watcher error: {0}")]
    Watcher(#[from] notify::Error),

    /// Error related to filesystem operations.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Error related to transcoding processes.
    #[error("Transcoding error: {0}")]
    Transcoding(String),

    /// A known required binary could not be resolved.
    #[error("Required extraction tool is unavailable: {0}")]
    ToolUnavailable(String),

    /// A provider returned data that cannot be used as a generated output.
    #[error("Extractor returned invalid output.")]
    InvalidProcessingOutput,

    /// Error related to streaming operations.
    #[error("Streaming error: {0}")]
    Streaming(String),

    /// Error when a resource (file, folder, tag) is not found.
    #[error("Requested resource not found: {0}")]
    NotFound(String),

    /// Error related to asset processing or format extraction.
    #[error("Format signature '{0}' is not supported by any known Capability Provider.")]
    FormatNotSupported(String),

    /// Error when a file format is not supported.
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    /// Timeout during a heavy processing task (FFmpeg, Image extraction, etc).
    #[error("Extraction process timed out.")]
    ExtractionProcessTimeout,

    /// Generic internal error for unexpected state.
    #[error("Internal state error: {0}")]
    Internal(String),

    /// Generic validation for input data.
    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    /// Error when access to a resource is forbidden.
    #[error("Access forbidden: {0}")]
    Forbidden(String),

    /// Error related to the internal Event Bus.
    #[error("Event Bus error: {0}")]
    EventBus(String),

    /// Generic error with a custom message.
    #[error("Application error: {0}")]
    Generic(String),

    /// Error when a file is identified but no provider can process it further.
    #[error("File identified but no further processing is available.")]
    NoResolutionLimit,
}

/// Unit tests for the AppError enum.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_error_display() {
        let error = AppError::NotFound("asset_123".to_string());
        assert_eq!(
            format!("{}", error),
            "Requested resource not found: asset_123"
        );

        let error = AppError::Generic("Something failed".to_string());
        assert_eq!(format!("{}", error), "Application error: Something failed");
    }

    #[test]
    fn test_app_error_from_io() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let error: AppError = io_error.into();
        match error {
            AppError::Io(_) => (),
            _ => panic!("Expected AppError::Io"),
        }
    }
}
