//! Host-side access to a single published thumbnail for a worker attempt.

use crate::core::AppResult;
use async_trait::async_trait;

/// Identity captured when the host verifies a package and its preview consent.
pub struct PluginPreviewApproval {
    pub catalog_id: String,
    pub plugin_id: String,
    pub publisher_fingerprint: [u8; 32],
    pub package_digest: [u8; 32],
    pub activation_epoch: String,
}

/// Keeps preview consent stable during a short catalog operation.
pub trait HeldPluginPreviewApproval: Send {}

/// Limit on the encoded native thumbnail carried by one attempt.
pub const MAX_PLUGIN_PREVIEW_BYTES: usize = 1024 * 1024;

/// Limit on a single broker response before transport framing.
pub const MAX_PLUGIN_PREVIEW_CHUNK: usize = 64 * 1024;

/// Reads an app-owned generated artifact without resolving any guest-supplied paths.
#[async_trait]
pub trait PluginPreviewStorage: Send + Sync {
    /// Opens the stored cache name and reads bounded encoded bytes.
    ///
    /// # Errors
    /// Rejects invalid names, unavailable files and outputs exceeding the fixed cap.
    async fn read_thumbnail(&self, file_name: &str) -> AppResult<Vec<u8>>;
}
