CREATE TABLE import_pairings (
    id TEXT PRIMARY KEY,
    server_key TEXT NOT NULL,
    expires_at INTEGER NOT NULL,
    phase TEXT NOT NULL CHECK (phase IN ('invited', 'pending', 'approved', 'claimed')),
    verifier TEXT UNIQUE,
    client_name TEXT,
    scopes INTEGER NOT NULL DEFAULT 0,
    roots TEXT NOT NULL DEFAULT '[]'
);

CREATE TABLE import_clients (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    server_key TEXT NOT NULL,
    verifier TEXT UNIQUE,
    expires_at INTEGER NOT NULL,
    grant_version INTEGER NOT NULL DEFAULT 1,
    scopes INTEGER NOT NULL,
    roots TEXT NOT NULL
);
