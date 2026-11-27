-- Attach annotations to the recovery point so namespace cleanup also covers Undo.
CREATE TABLE folder_recovery_plugin_asset_data (
    folder_id TEXT NOT NULL REFERENCES folder_recovery(folder_id) ON DELETE CASCADE,
    asset_id TEXT NOT NULL,
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
    PRIMARY KEY (folder_id, asset_id, plugin_id, publisher_fingerprint, field_id)
) STRICT;

-- Only v1 snapshots included annotations. Leave unknown and invalid snapshots for recovery repair.
WITH snapshots AS (
    SELECT folder_id, CASE WHEN json_valid(snapshot_json) THEN snapshot_json ELSE '{}' END AS content
    FROM folder_recovery
)
INSERT INTO folder_recovery_plugin_asset_data
    (folder_id, asset_id, plugin_id, publisher_fingerprint, field_id, schema_version, revision, value_json)
SELECT snapshots.folder_id, json_extract(field.value, '$.asset_id'), json_extract(field.value, '$.plugin_id'),
       json_extract(field.value, '$.publisher_fingerprint'), json_extract(field.value, '$.field_id'),
       json_extract(field.value, '$.schema_version'), json_extract(field.value, '$.revision'),
       json_extract(field.value, '$.value_json')
FROM snapshots, json_each(snapshots.content, '$.tables[7].rows') AS field
WHERE json_extract(snapshots.content, '$.version') = 1
  AND json_extract(snapshots.content, '$.tables[7].table') = 'plugin_asset_data';

UPDATE folder_recovery
SET snapshot_json = json_set(snapshot_json, '$.tables[7].rows', json('[]'))
WHERE folder_id IN (SELECT folder_id FROM folder_recovery_plugin_asset_data);

CREATE INDEX idx_plugin_asset_data_owner ON plugin_asset_data(plugin_id, publisher_fingerprint);
CREATE INDEX idx_recovery_plugin_data_owner ON folder_recovery_plugin_asset_data(plugin_id, publisher_fingerprint);
CREATE INDEX idx_plugin_derived_data_owner ON plugin_derived_results(plugin_id, lower(hex(publisher_fingerprint)));
