CREATE TABLE library_operation_receipts (
    operation_id TEXT PRIMARY KEY,
    entity_id TEXT NOT NULL,
    operation_kind TEXT NOT NULL,
    cleanup_assets_json TEXT NOT NULL DEFAULT '[]',
    cleanup_pending INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE library_file_identities (
    asset_id TEXT PRIMARY KEY REFERENCES assets(id) ON DELETE CASCADE,
    device TEXT NOT NULL,
    object TEXT NOT NULL
);

ALTER TABLE library_operation_receipts ADD COLUMN created_identity_json TEXT;

ALTER TABLE folder_recovery ADD COLUMN byte_count INTEGER NOT NULL DEFAULT -1;

ALTER TABLE library_file_identities ADD COLUMN birth TEXT;

ALTER TABLE library_operation_receipts ADD COLUMN confirmed_path TEXT NOT NULL DEFAULT '';

CREATE TABLE library_folder_identities (folder_id TEXT PRIMARY KEY REFERENCES folders(id) ON DELETE CASCADE, device TEXT NOT NULL, object TEXT NOT NULL, birth TEXT);
