//! Manifest preflight shared by import clients and the future admission handler.
//! Catalog authorization, storage reservations and content checks follow preflight.

use crate::core::{
    error::{AppError, AppResult},
    ledger::discovery_metadata::normalize_source_url,
    library::policy::validate_name,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub mod catalog;
pub mod destinations;
pub mod pairing;
pub mod publication;
pub mod staging;

/// Resource limits supplied by the host, never by the import request.
pub struct ImportLimits {
    pub max_items: usize,
    pub max_tags_per_item: usize,
    pub max_item_bytes: u64,
    pub max_batch_bytes: u64,
}

/// A request to import files into one explicit or subsequently resolved default folder.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportManifest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tag_ids: Vec<String>,
    pub items: Vec<ImportItem>,
}

/// Client identity and declared attributes of one file. The host owns physical paths.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportItem {
    pub item_id: String,
    pub file_name: String,
    pub byte_length: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tag_ids: Vec<String>,
}

impl ImportManifest {
    /// Produces deterministic per-item tag sets while preserving item order and destination intent.
    /// An omitted folder stays unresolved until the server checks the client's grants.
    ///
    /// # Errors
    /// Returns validation errors for invalid declarations, duplicate item IDs and exceeded limits.
    pub fn normalize(mut self, limits: &ImportLimits) -> AppResult<Self> {
        if self.items.is_empty() || self.items.len() > limits.max_items {
            return Err(AppError::ValidationFailed(
                "Invalid import item count".into(),
            ));
        }
        if self
            .folder_id
            .as_ref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(AppError::ValidationFailed(
                "Empty destination identifier".into(),
            ));
        }
        normalize_tags(&mut self.tag_ids, limits.max_tags_per_item)?;
        let mut item_ids = HashSet::new();
        let mut total_bytes = 0_u64;
        for item in &mut self.items {
            let id = &item.item_id;
            if id.is_empty()
                || id.len() > 128
                || !id
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
                || !item_ids.insert(id.as_str())
            {
                return Err(AppError::ValidationFailed(
                    "Invalid or duplicate import item ID".into(),
                ));
            }
            validate_name(&item.file_name)?;
            // JSON clients must be able to acknowledge the exact byte offset.
            if item.byte_length > limits.max_item_bytes.min(9_007_199_254_740_991) {
                return Err(AppError::ValidationFailed(
                    "Import item exceeds byte limit".into(),
                ));
            }
            total_bytes = total_bytes
                .checked_add(item.byte_length)
                .filter(|total| *total <= limits.max_batch_bytes)
                .ok_or_else(|| {
                    AppError::ValidationFailed("Import batch exceeds byte limit".into())
                })?;
            if let Some(origin) = &item.source_url {
                item.source_url = normalize_source_url(origin)?;
            }
            normalize_tags(&mut item.tag_ids, limits.max_tags_per_item)?;
            item.tag_ids.extend(self.tag_ids.iter().cloned());
            item.tag_ids.sort_unstable();
            item.tag_ids.dedup();
            if item.tag_ids.len() > limits.max_tags_per_item {
                return Err(AppError::ValidationFailed(
                    "Too many tags on import item".into(),
                ));
            }
        }
        self.tag_ids.clear();
        Ok(self)
    }
}

fn normalize_tags(tags: &mut Vec<String>, limit: usize) -> AppResult<()> {
    if tags.len() > limit || tags.iter().any(|tag| tag.trim().is_empty()) {
        return Err(AppError::ValidationFailed("Invalid import tags".into()));
    }
    tags.sort_unstable();
    tags.dedup();
    Ok(())
}

#[cfg(test)]
mod tests;
