ALTER TABLE processing_tasks ADD COLUMN thumbnail_artifact TEXT;
ALTER TABLE processing_tasks ADD COLUMN thumbnail_digest BLOB CHECK (
    (thumbnail_artifact IS NULL AND thumbnail_digest IS NULL)
    OR (thumbnail_artifact IS NOT NULL AND thumbnail_digest IS NOT NULL AND length(thumbnail_digest) = 32)
);

-- Regenerable data. Folder recovery snapshots include durable plugin fields only.
CREATE TABLE plugin_derived_results (
    asset_id TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    plugin_id TEXT NOT NULL CHECK (length(plugin_id) BETWEEN 1 AND 128),
    publisher_fingerprint BLOB NOT NULL CHECK (length(publisher_fingerprint) = 32),
    result_id TEXT NOT NULL CHECK (length(result_id) BETWEEN 1 AND 32),
    package_digest BLOB NOT NULL CHECK (length(package_digest) = 32),
    schema_version INTEGER NOT NULL CHECK (schema_version BETWEEN 1 AND 4294967295),
    source_revision BLOB NOT NULL CHECK (length(source_revision) = 32),
    value_json TEXT NOT NULL CHECK (
        length(CAST(value_json AS BLOB)) <= 65536 AND json_valid(value_json) AND json_type(value_json) = 'object'
    ),
    PRIMARY KEY (asset_id, plugin_id, publisher_fingerprint, result_id)
) STRICT;
