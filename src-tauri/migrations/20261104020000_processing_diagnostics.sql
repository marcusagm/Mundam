-- Preserve existing outcomes; historical logs cannot be safely classified retroactively.
ALTER TABLE processing_tasks ADD COLUMN issue_code TEXT;
