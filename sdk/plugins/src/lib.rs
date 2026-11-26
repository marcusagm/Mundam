//! Mundam's declarative plugin authoring contract, shared through types and JSON.
//!
//! Validation performs no I/O and grants no permissions. A package manager can
//! supply verified bytes and reuse this policy without depending on desktop IPC.

mod icon;
mod model;
pub mod package;
mod validation;

use std::fmt;

pub use icon::validate_icon;
pub use model::*;
pub use semver::Version;
use validation::ApiInterval;
pub use validation::{parse_manifest, valid_namespace};

/// JSON Schema for editor integration and structural validation.
pub const MANIFEST_SCHEMA: &str = include_str!("../v1/manifest.schema.json");

/// Maximum bytes of the raw UTF-8 document before deserialization.
pub const MAX_MANIFEST_BYTES: usize = 64 * 1024;

/// Maximum bytes of the compressed PNG resource.
pub const MAX_ICON_BYTES: usize = 256 * 1024;

/// A validation diagnostic with a stable field path for CLI and host clients.
#[derive(Debug)]
pub struct ValidationError {
    pub field: &'static str,
    pub message: String,
}

impl ValidationError {
    fn new(field: &'static str, message: impl Into<String>) -> Self {
        Self {
            field,
            message: message.into(),
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.field, self.message)
    }
}

impl std::error::Error for ValidationError {}

/// The actual process target known to the host. Unknown native targets fail closed.
#[derive(Clone, Copy, Debug)]
pub struct Executor {
    platform: Platform,
    architecture: Architecture,
}

impl Executor {
    /// Builds a concrete target from Rust's OS and architecture vocabulary.
    pub fn from_system(platform: &str, architecture: &str) -> Option<Self> {
        let platform = match platform {
            "windows" => Platform::Windows,
            "macos" => Platform::Mac,
            "linux" => Platform::Linux,
            _ => return None,
        };
        let architecture = match architecture {
            "x86_64" => Architecture::X64,
            "aarch64" => Architecture::Arm64,
            "arm" => Architecture::Arm,
            _ => return None,
        };
        Some(Self {
            platform,
            architecture,
        })
    }
}

/// Parsed declaration ready for compatibility checks and a renderer projection.
#[derive(Debug)]
pub struct ValidatedManifest {
    manifest: Manifest,
    api: ApiInterval,
}

impl ValidatedManifest {
    /// Provides read-only access without allowing callers to invalidate the checks.
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    /// Evaluates target pairs against one executor and checks the stable API version.
    pub fn supports(&self, executor: Executor, api_version: &Version) -> bool {
        self.api.contains(api_version)
            && self.manifest.targets.iter().any(|target| {
                (target.platform == Platform::All || target.platform == executor.platform)
                    && (target.architecture == Architecture::All
                        || target.architecture == executor.architecture)
            })
    }
}
