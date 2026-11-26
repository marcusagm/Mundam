ALTER TABLE installed_plugins DROP COLUMN enabled;

CREATE TABLE plugin_catalog_activations (
    catalog_id TEXT NOT NULL CHECK (length(catalog_id) BETWEEN 1 AND 128),
    plugin_id TEXT NOT NULL REFERENCES installed_plugins (id) ON DELETE CASCADE,
    package_digest BLOB NOT NULL CHECK (length(package_digest) = 32),
    permissions TEXT NOT NULL CHECK (json_valid(permissions) AND json_type(permissions) = 'array'),
    PRIMARY KEY (catalog_id, plugin_id)
) STRICT;

CREATE INDEX plugin_catalog_activations_by_plugin ON plugin_catalog_activations (plugin_id);
