//! Safe extraction-tool facts; observations never change user pause intent.

use crate::core::AppResult;
use async_trait::async_trait;
use serde::Serialize;

/// Only known executables participate in automatic dependency suspension.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionTool {
    Ffmpeg,
    Ffprobe,
    Assimp,
}

impl ExtractionTool {
    /// Stable inventory, independent of assets or backlog size.
    pub const ALL: [Self; 3] = [Self::Ffmpeg, Self::Ffprobe, Self::Assimp];
    /// Resolves the executable's canonical name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ffmpeg => "ffmpeg",
            Self::Ffprobe => "ffprobe",
            Self::Assimp => "assimp",
        }
    }
    /// Recognizes typed dependency failures without interpreting human-readable messages.
    pub fn from_error(error: &crate::core::AppError) -> Option<Self> {
        Self::from_issue(
            crate::core::processing::diagnostics::ProcessingIssue::from_error(error).as_str(),
        )
    }
    /// Maps a persisted typed diagnostic without interpreting private error messages.
    pub fn from_issue(issue: &str) -> Option<Self> {
        match issue {
            "ffmpeg_unavailable" => Some(Self::Ffmpeg),
            "ffprobe_unavailable" => Some(Self::Ffprobe),
            "assimp_unavailable" => Some(Self::Assimp),
            _ => None,
        }
    }
}

/// Last observed availability, not a guarantee that a future process will succeed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolAvailability {
    Unknown,
    Available,
    Unavailable,
}

/// Bounded public facts exclude binary paths and subprocess output.
#[derive(Clone, Debug, Serialize)]
pub struct ExtractionToolStatus {
    pub tool: ExtractionTool,
    pub state: ToolAvailability,
    pub checked_at: Option<i64>,
    /// Fences a probe that started before a newer extraction failure.
    #[serde(skip)]
    pub revision: i64,
}

/// A version-command probe, associated with the revision captured before it began.
pub struct ExtractionToolCheck {
    pub tool: ExtractionTool,
    pub available: bool,
    pub expected_revision: i64,
}

/// Infrastructure executes bounded probes away from the async executor.
#[async_trait]
pub trait ExtractionToolProbe: Send + Sync {
    /// Tests only the supplied tool, without input assets.
    /// # Errors
    /// Propagates infrastructure failures; unavailable tools return false.
    async fn available(&self, tool: ExtractionTool) -> AppResult<bool>;
}
