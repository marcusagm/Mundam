//! Pure validation of ordered taxonomy import contracts.

use super::command::ImportTagsPayload;
use crate::core::error::{AppError, AppResult};
use std::collections::HashSet;

/// Rejects ambiguous outlines before any destructive replacement can occur.
///
/// # Errors
/// Returns validation errors for empty lists, duplicate names, forward parent
/// references, or unsupported color values.
pub fn validate_import(payload: &ImportTagsPayload) -> AppResult<()> {
    if payload.entries.is_empty() {
        return Err(AppError::ValidationFailed("Enter at least one tag.".into()));
    }
    let mut names = HashSet::new();
    for (index, entry) in payload.entries.iter().enumerate() {
        if entry.name.trim().is_empty() || entry.name.contains(['\n', '\r']) {
            return Err(AppError::ValidationFailed(
                "Tag names must be nonempty single lines.".into(),
            ));
        }
        if !names.insert(entry.name.trim()) {
            return Err(AppError::ValidationFailed(format!(
                "Duplicate tag: {}",
                entry.name
            )));
        }
        if entry.parent_index.is_some_and(|parent| parent >= index) {
            return Err(AppError::ValidationFailed(
                "Parents must precede children.".into(),
            ));
        }
        if let Some(color) = &entry.color {
            let digits = color.strip_prefix('#').unwrap_or("");
            if ![3, 4, 6, 8].contains(&digits.len())
                || !digits.bytes().all(|digit| digit.is_ascii_hexdigit())
            {
                return Err(AppError::ValidationFailed(format!(
                    "Invalid hexadecimal color: {color}"
                )));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ledger::command::ImportTagEntry;

    #[test]
    fn rejects_empty_duplicate_forward_parent_and_invalid_color_outlines() {
        let mut payload = ImportTagsPayload {
            entries: vec![],
            replace_existing: true,
        };
        assert!(validate_import(&payload).is_err());
        payload.entries.push(ImportTagEntry {
            name: "Valid".into(),
            parent_index: None,
            color: None,
        });
        assert!(validate_import(&payload).is_ok());
        payload.entries[0].parent_index = Some(0);
        assert!(validate_import(&payload).is_err());
        payload.entries[0].parent_index = None;
        payload.entries[0].color = Some("not-a-color".into());
        assert!(validate_import(&payload).is_err());
        payload.entries[0].color = Some("#abcd".into());
        assert!(validate_import(&payload).is_ok());
        payload.entries.push(payload.entries[0].clone());
        assert!(validate_import(&payload).is_err());
    }
}
