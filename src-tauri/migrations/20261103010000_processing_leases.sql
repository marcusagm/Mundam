-- Opaque ownership fences attempts even when generation and source remain unchanged.
ALTER TABLE processing_tasks ADD COLUMN lease_id TEXT;
ALTER TABLE processing_tasks ADD COLUMN lease_expires_at INTEGER;
ALTER TABLE processing_tasks ADD COLUMN preview_path TEXT;
CREATE INDEX idx_processing_tasks_lease ON processing_tasks(kind, lease_expires_at);
