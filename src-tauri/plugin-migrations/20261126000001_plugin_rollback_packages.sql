ALTER TABLE installed_plugins ADD COLUMN package_revision TEXT NOT NULL DEFAULT '';
UPDATE installed_plugins SET package_revision = lower(hex(randomblob(16)));

CREATE TABLE plugin_rollback_packages (
    plugin_id TEXT NOT NULL PRIMARY KEY REFERENCES installed_plugins (id) ON DELETE CASCADE,
    version TEXT NOT NULL,
    package_digest BLOB NOT NULL CHECK (length(package_digest) = 32),
    package BLOB NOT NULL CHECK (length(package) BETWEEN 1 AND 327799)
) STRICT;
