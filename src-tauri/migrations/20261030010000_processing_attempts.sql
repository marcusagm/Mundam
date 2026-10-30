-- Durable requests and bounded attempt outcomes; execution leases are a later stage.
CREATE TABLE processing_tasks (
    asset_id TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('thumbnail', 'colors', 'fingerprint')),
    state TEXT NOT NULL CHECK (state IN ('pending', 'succeeded', 'retry_wait', 'failed', 'skipped')),
    generation INTEGER NOT NULL DEFAULT 0,
    source_stamp TEXT NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 0,
    next_attempt_at INTEGER,
    last_error TEXT,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (asset_id, kind)
);
CREATE INDEX idx_processing_tasks_ready ON processing_tasks(kind, state, next_attempt_at);

-- Empty legacy paths represented both errors and unsupported providers as success.
-- Give these assets one real attempt under the bounded retry policy.
UPDATE assets SET thumbnail_path = NULL, state = 'Indexed'
WHERE thumbnail_path = '' AND deleted_at IS NULL AND LOWER(state) != 'offline';
