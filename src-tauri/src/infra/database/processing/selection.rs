//! Canonical eligibility predicates shared by processing reads and atomic admissions.

use crate::core::processing::ProcessingKind;

pub(crate) const SOURCE_STAMP: &str =
    "a.path || '|' || a.file_size || '|' || COALESCE(a.modified_at, '')";
pub(crate) const ACTIVE_ASSET: &str =
    "a.deleted_at IS NULL AND LOWER(a.state) NOT IN ('trashed', 'offline')";

pub(crate) fn pending_selection(kind: ProcessingKind) -> String {
    let missing_result = missing_result(kind);
    let dependency = dependency(kind);
    format!(
        "FROM assets a
         LEFT JOIN processing_tasks task ON task.asset_id = a.id AND task.kind = ?1
         LEFT JOIN duplicate_fingerprints df ON df.asset_id = a.id
         WHERE {ACTIVE_ASSET} {dependency}
         AND ({missing_result} OR task.state IN ('pending', 'retry_wait') OR task.source_stamp != ({SOURCE_STAMP}))
         AND (task.asset_id IS NULL OR task.source_stamp != ({SOURCE_STAMP})
              OR task.state = 'pending'
              OR (task.state = 'succeeded' AND {missing_result})
              OR (task.state = 'retry_wait' AND task.next_attempt_at <= unixepoch()))
         AND (task.lease_id IS NULL OR task.lease_expires_at <= unixepoch()
              OR task.source_stamp != ({SOURCE_STAMP}))"
    )
}

pub(crate) fn missing_result(kind: ProcessingKind) -> &'static str {
    match kind {
        ProcessingKind::Thumbnail => "(a.thumbnail_path IS NULL OR a.thumbnail_path = '')",
        ProcessingKind::Colors => "NOT EXISTS (SELECT 1 FROM asset_colors WHERE asset_id = a.id)",
        ProcessingKind::Fingerprint => "(df.asset_id IS NULL OR df.content_hash LIKE 'pending_%' OR df.content_hash LIKE 'hash_%' OR df.fingerprint_version < 5)",
    }
}

pub(crate) fn dependency(kind: ProcessingKind) -> &'static str {
    if kind == ProcessingKind::Colors {
        "AND a.thumbnail_path IS NOT NULL AND a.thumbnail_path != ''"
    } else {
        ""
    }
}
