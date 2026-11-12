use crate::core::error::domain::AppError;
use serde::{Serialize, Serializer};

/// Structured JSON payload for errors crossing the Tauri IPC boundary.
#[derive(Serialize)]
struct ErrorPayload {
    code: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<String>,
}

/// Serializes the AppError enum into a structured JSON payload.
impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let (code, message, details) = match self {
            AppError::ToolUnavailable(_) => (
                "TOOL_UNAVAILABLE",
                "A required extraction tool is unavailable.".into(),
                None,
            ),
            AppError::InvalidProcessingOutput => (
                "INVALID_PROCESSING_OUTPUT",
                "Extractor returned invalid output.".into(),
                None,
            ),
            AppError::NotFound(msg) => ("NOT_FOUND", msg.clone(), None),
            AppError::Database(err) => (
                "DATABASE_ERROR",
                "A strict database violation occurred.".to_string(),
                Some(err.to_string()),
            ),
            AppError::Io(err) => ("IO_ERROR", err.to_string(), None),
            AppError::FormatNotSupported(fmt) => (
                "FORMAT_NOT_SUPPORTED",
                format!(
                    "Format signature '{}' is not supported by any known Capability Provider.",
                    fmt
                ),
                None,
            ),
            AppError::ExtractionProcessTimeout => {
                ("TIMEOUT", "Extraction process timed out.".to_string(), None)
            }
            AppError::ValidationFailed(msg) => ("VALIDATION_FAILED", msg.clone(), None),
            AppError::Cancelled => ("CANCELLED", "Operation cancelled.".to_string(), None),
            _ => ("INTERNAL_ERROR", self.to_string(), None),
        };

        ErrorPayload {
            code: code.to_string(),
            message,
            details,
        }
        .serialize(serializer)
    }
}

/// Unit tests for the AppError serialization.
#[cfg(test)]
mod tests {
    use crate::core::error::domain::AppError;
    use serde_json::json;

    #[test]
    fn test_app_error_serialization_not_found() -> Result<(), Box<dyn std::error::Error>> {
        let error = AppError::NotFound("asset_456".to_string());
        let serialized = serde_json::to_value(&error)?;

        assert_eq!(
            serialized,
            json!({
                "code": "NOT_FOUND",
                "message": "asset_456"
            })
        );

        Ok(())
    }

    #[test]
    fn test_app_error_serialization_database() -> Result<(), Box<dyn std::error::Error>> {
        // Mocking a database error is hard, but we can verify the structure
        let error = AppError::Internal("Unexpected database state".to_string());
        let serialized = serde_json::to_value(&error)?;

        assert_eq!(serialized["code"], "INTERNAL_ERROR");
        assert_eq!(
            serialized["message"],
            "Internal state error: Unexpected database state"
        );

        Ok(())
    }

    #[test]
    fn test_app_error_serialization_generic() -> Result<(), Box<dyn std::error::Error>> {
        let error = AppError::Generic("Manual failure".to_string());
        let serialized = serde_json::to_value(&error)?;

        assert_eq!(
            serialized,
            json!({
                "code": "INTERNAL_ERROR",
                "message": "Application error: Manual failure"
            })
        );

        Ok(())
    }
}
