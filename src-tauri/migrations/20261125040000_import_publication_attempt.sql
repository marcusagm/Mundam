ALTER TABLE import_publication_receipts ADD COLUMN attempt_id TEXT;
CREATE UNIQUE INDEX idx_import_publication_attempt
    ON import_publication_receipts(attempt_id)
    WHERE attempt_id IS NOT NULL;
