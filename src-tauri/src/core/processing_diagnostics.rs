//! Safe processing causes and explicit retry intent, independent of raw extractor logs.

use crate::core::AppError;
use serde::{Deserialize, Serialize};

/// Stable causes are grouped without exposing filenames, command lines or database errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessingIssue {
    ProviderMissing,
    CapabilityMissing,
    ToolUnavailable,
    FfmpegUnavailable,
    FfprobeUnavailable,
    AssimpUnavailable,
    InputUnavailable,
    InvalidOutput,
    Timeout,
    StorageFailure,
    ExtractionFailed,
    UnknownFailure,
    UnknownUnsupported,
}

impl ProcessingIssue {
    /// Finite whitelist also bounds diagnostic payload size when legacy data is malformed.
    pub const CODES: [&'static str; 13] = [
        "provider_missing",
        "capability_missing",
        "tool_unavailable",
        "ffmpeg_unavailable",
        "ffprobe_unavailable",
        "assimp_unavailable",
        "input_unavailable",
        "invalid_output",
        "timeout",
        "storage_failure",
        "extraction_failed",
        "unknown_failure",
        "unknown_unsupported",
    ];

    /// Stable persisted representation; untrusted strings never become UI messages.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ProviderMissing => "provider_missing",
            Self::CapabilityMissing => "capability_missing",
            Self::ToolUnavailable => "tool_unavailable",
            Self::FfmpegUnavailable => "ffmpeg_unavailable",
            Self::FfprobeUnavailable => "ffprobe_unavailable",
            Self::AssimpUnavailable => "assimp_unavailable",
            Self::InputUnavailable => "input_unavailable",
            Self::InvalidOutput => "invalid_output",
            Self::Timeout => "timeout",
            Self::StorageFailure => "storage_failure",
            Self::ExtractionFailed => "extraction_failed",
            Self::UnknownFailure => "unknown_failure",
            Self::UnknownUnsupported => "unknown_unsupported",
        }
    }

    /// Uses error types, never heuristics over paths or extractor stderr.
    pub fn from_error(error: &AppError) -> Self {
        match error {
            AppError::ToolUnavailable(tool) => match tool.as_str() {
                "ffmpeg" => Self::FfmpegUnavailable,
                "ffprobe" => Self::FfprobeUnavailable,
                "assimp" => Self::AssimpUnavailable,
                _ => Self::ToolUnavailable,
            },
            AppError::InvalidProcessingOutput => Self::InvalidOutput,
            AppError::ExtractionProcessTimeout => Self::Timeout,
            AppError::Database(_) | AppError::Migration(_) => Self::StorageFailure,
            AppError::Io(_) | AppError::NotFound(_) => Self::InputUnavailable,
            AppError::FormatNotSupported(_)
            | AppError::UnsupportedFormat(_)
            | AppError::NoResolutionLimit => Self::CapabilityMissing,
            _ => Self::ExtractionFailed,
        }
    }
}

/// Only terminal failed or unsupported work is reset; scheduled retries keep their backoff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessingRetryScope {
    Failed,
    Unsupported,
}

/// An aggregate cause for the current source, never a list of asset identities.
#[derive(Clone, Serialize)]
pub struct ProcessingDiagnostic {
    pub issue: String,
    pub state: String,
    pub count: u64,
}

/// Counts actual changes, including comparison jobs only for the duplicates worker.
#[derive(Default, Serialize)]
pub struct ProcessingRetryCounts {
    pub assets: u64,
    pub comparisons: u64,
}
