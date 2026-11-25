-- Replay records outlive the published asset and never recreate a removed file.
CREATE TABLE import_publication_receipts (
    batch_id TEXT NOT NULL,
    item_id TEXT NOT NULL,
    fingerprint BLOB NOT NULL CHECK(length(fingerprint) = 32),
    asset_id TEXT NOT NULL,
    file_name TEXT NOT NULL,
    published_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (batch_id, item_id)
);
