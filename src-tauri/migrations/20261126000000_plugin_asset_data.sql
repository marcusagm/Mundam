-- Catalog-owned data. Package changes in the profile registry do not delete these rows.
CREATE TABLE plugin_asset_data (
    asset_id TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    plugin_id TEXT NOT NULL CHECK (length(plugin_id) BETWEEN 1 AND 128),
    publisher_fingerprint TEXT NOT NULL CHECK (
        length(publisher_fingerprint) = 64 AND publisher_fingerprint NOT GLOB '*[^0-9a-f]*'
    ),
    field_id TEXT NOT NULL CHECK (length(field_id) BETWEEN 1 AND 32),
    schema_version INTEGER NOT NULL CHECK (schema_version BETWEEN 1 AND 4294967295),
    revision INTEGER NOT NULL CHECK (revision > 0),
    value_json TEXT NOT NULL CHECK (
        length(CAST(value_json AS BLOB)) <= 65536 AND json_valid(value_json)
    ),
    PRIMARY KEY (asset_id, plugin_id, publisher_fingerprint, field_id)
) STRICT;
