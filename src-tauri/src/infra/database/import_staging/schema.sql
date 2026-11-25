CREATE TABLE IF NOT EXISTS import_batches (
    id TEXT PRIMARY KEY,
    catalog TEXT NOT NULL,
    owner TEXT NOT NULL,
    request_key TEXT NOT NULL,
    manifest TEXT NOT NULL,
    folder_id TEXT NOT NULL,
    expires INTEGER NOT NULL,
    reserved_bytes INTEGER NOT NULL,
    cancelled INTEGER NOT NULL DEFAULT 0 CHECK (cancelled IN (0, 1)),
    UNIQUE(catalog, owner, request_key)
);

CREATE TABLE IF NOT EXISTS import_parts (
    batch_id TEXT NOT NULL REFERENCES import_batches(id) ON DELETE CASCADE,
    sequence INTEGER NOT NULL,
    item_id TEXT NOT NULL,
    part_number INTEGER NOT NULL,
    digest TEXT NOT NULL,
    content BLOB NOT NULL,
    PRIMARY KEY(batch_id, sequence)
);

CREATE TABLE IF NOT EXISTS import_publication_claims (
    batch_id TEXT PRIMARY KEY REFERENCES import_batches(id) ON DELETE CASCADE,
    item_id TEXT NOT NULL,
    attempt_id TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS import_published_items (
    batch_id TEXT NOT NULL REFERENCES import_batches(id) ON DELETE CASCADE,
    item_id TEXT NOT NULL,
    asset_id TEXT NOT NULL,
    file_name TEXT NOT NULL,
    PRIMARY KEY (batch_id, item_id)
);
