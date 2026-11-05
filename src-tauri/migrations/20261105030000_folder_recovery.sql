CREATE TABLE folder_recovery (
    folder_id TEXT PRIMARY KEY,
    original_path TEXT NOT NULL,
    archive_path TEXT NOT NULL,
    snapshot_json TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
