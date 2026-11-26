CREATE TABLE plugin_publishers (
    namespace TEXT NOT NULL PRIMARY KEY CHECK (length(namespace) BETWEEN 3 AND 128),
    fingerprint BLOB NOT NULL UNIQUE CHECK (length(fingerprint) = 32),
    revoked_at INTEGER,
    UNIQUE (namespace, fingerprint)
) STRICT;

CREATE TABLE installed_plugins (
    id TEXT NOT NULL PRIMARY KEY,
    publisher TEXT NOT NULL,
    publisher_fingerprint BLOB NOT NULL CHECK (length(publisher_fingerprint) = 32),
    version TEXT NOT NULL,
    name TEXT NOT NULL,
    package_digest BLOB NOT NULL UNIQUE CHECK (length(package_digest) = 32),
    package BLOB NOT NULL CHECK (length(package) BETWEEN 1 AND 327799),
    enabled INTEGER NOT NULL DEFAULT 0 CHECK (enabled IN (0, 1)),
    FOREIGN KEY (publisher, publisher_fingerprint) REFERENCES plugin_publishers (namespace, fingerprint)
) STRICT;
