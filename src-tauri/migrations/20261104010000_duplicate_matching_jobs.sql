-- Two coalesced jobs retain intent independently of process memory and notifications.
CREATE TABLE duplicate_matching_revision (id INTEGER PRIMARY KEY CHECK(id = 1), revision INTEGER NOT NULL, publication_sequence INTEGER NOT NULL DEFAULT 0);
INSERT INTO duplicate_matching_revision(id, revision) VALUES (1, 1);
CREATE TABLE duplicate_matching_jobs (
    kind TEXT PRIMARY KEY CHECK(kind IN ('exact_matching', 'visual_matching')),
    requested_revision INTEGER NOT NULL DEFAULT 1,
    completed_revision INTEGER NOT NULL DEFAULT 0,
    state TEXT NOT NULL DEFAULT 'pending' CHECK(state IN ('pending', 'succeeded', 'retry_wait', 'failed')),
    automatic_requested INTEGER NOT NULL DEFAULT 1,
    manual_run_id TEXT,
    cancel_requested INTEGER NOT NULL DEFAULT 0,
    lease_id TEXT,
    lease_expires_at INTEGER,
    lease_manual_run_id TEXT,
    policy_json TEXT,
    attempts INTEGER NOT NULL DEFAULT 0,
    next_attempt_at INTEGER,
    last_error TEXT
);
INSERT INTO duplicate_matching_jobs(kind) VALUES ('exact_matching'), ('visual_matching');

CREATE TRIGGER matching_fingerprint_insert AFTER INSERT ON duplicate_fingerprints

BEGIN
    UPDATE duplicate_matching_revision SET revision = revision + 1 WHERE id = 1;
    UPDATE duplicate_matching_jobs SET requested_revision = (SELECT revision FROM duplicate_matching_revision WHERE id = 1),
        automatic_requested = 1, state = 'pending', attempts = 0, next_attempt_at = NULL, last_error = NULL;
END;

CREATE TRIGGER matching_fingerprint_update AFTER UPDATE ON duplicate_fingerprints
WHEN OLD.content_hash IS NOT NEW.content_hash OR OLD.perceptual_hash IS NOT NEW.perceptual_hash
    OR OLD.block_hash IS NOT NEW.block_hash OR OLD.thumb_hash IS NOT NEW.thumb_hash
    OR OLD.width IS NOT NEW.width OR OLD.height IS NOT NEW.height OR OLD.file_size IS NOT NEW.file_size
    OR OLD.fingerprint_version IS NOT NEW.fingerprint_version
BEGIN
    UPDATE duplicate_matching_revision SET revision = revision + 1 WHERE id = 1;
    UPDATE duplicate_matching_jobs SET requested_revision = (SELECT revision FROM duplicate_matching_revision WHERE id = 1),
        automatic_requested = 1, state = 'pending', attempts = 0, next_attempt_at = NULL, last_error = NULL;
END;

CREATE TRIGGER matching_fingerprint_delete AFTER DELETE ON duplicate_fingerprints

BEGIN
    UPDATE duplicate_matching_revision SET revision = revision + 1 WHERE id = 1;
    UPDATE duplicate_matching_jobs SET requested_revision = (SELECT revision FROM duplicate_matching_revision WHERE id = 1),
        automatic_requested = 1, state = 'pending', attempts = 0, next_attempt_at = NULL, last_error = NULL;
END;

CREATE TRIGGER matching_policy_insert AFTER INSERT ON duplicate_rule_sets

BEGIN
    UPDATE duplicate_matching_revision SET revision = revision + 1 WHERE id = 1;
    UPDATE duplicate_matching_jobs SET requested_revision = (SELECT revision FROM duplicate_matching_revision WHERE id = 1),
        automatic_requested = 1, state = 'pending', attempts = 0, next_attempt_at = NULL, last_error = NULL;
END;

CREATE TRIGGER matching_policy_update AFTER UPDATE ON duplicate_rule_sets

BEGIN
    UPDATE duplicate_matching_revision SET revision = revision + 1 WHERE id = 1;
    UPDATE duplicate_matching_jobs SET requested_revision = (SELECT revision FROM duplicate_matching_revision WHERE id = 1),
        automatic_requested = 1, state = 'pending', attempts = 0, next_attempt_at = NULL, last_error = NULL;
END;

CREATE TRIGGER matching_policy_delete AFTER DELETE ON duplicate_rule_sets

BEGIN
    UPDATE duplicate_matching_revision SET revision = revision + 1 WHERE id = 1;
    UPDATE duplicate_matching_jobs SET requested_revision = (SELECT revision FROM duplicate_matching_revision WHERE id = 1),
        automatic_requested = 1, state = 'pending', attempts = 0, next_attempt_at = NULL, last_error = NULL;
END;

CREATE TRIGGER matching_asset_availability AFTER UPDATE OF state, deleted_at, path, file_size, modified_at ON assets
WHEN (OLD.deleted_at IS NULL) != (NEW.deleted_at IS NULL)
    OR (LOWER(OLD.state) IN ('trashed', 'offline')) != (LOWER(NEW.state) IN ('trashed', 'offline'))
    OR OLD.path IS NOT NEW.path OR OLD.file_size IS NOT NEW.file_size OR OLD.modified_at IS NOT NEW.modified_at
BEGIN
    UPDATE duplicate_matching_revision SET revision = revision + 1 WHERE id = 1;
    UPDATE duplicate_matching_jobs SET requested_revision = (SELECT revision FROM duplicate_matching_revision WHERE id = 1),
        automatic_requested = 1, state = 'pending', attempts = 0, next_attempt_at = NULL, last_error = NULL;
END;
