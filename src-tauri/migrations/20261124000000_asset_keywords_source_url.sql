ALTER TABLE assets ADD COLUMN keywords TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(keywords) AND json_type(keywords) = 'array');
ALTER TABLE assets ADD COLUMN source_url TEXT;
DROP TRIGGER assets_fts_after_insert;
DROP TRIGGER assets_fts_before_delete;
DROP TRIGGER assets_fts_after_update;
DROP TABLE assets_fts;
CREATE VIRTUAL TABLE assets_fts USING fts5(
    name,
    notes,
    keywords,
    content='assets',
    content_rowid='rowid',
    tokenize='trigram'
);

-- 2. Populate index from existing data.
INSERT INTO assets_fts(assets_fts) VALUES('rebuild');

-- 3. Sync triggers to keep FTS consistent with the assets table.

-- On INSERT: index the new asset's searchable fields.
CREATE TRIGGER assets_fts_after_insert AFTER INSERT ON assets BEGIN
    INSERT INTO assets_fts(rowid, name, notes, keywords)
    VALUES (new.rowid, new.name, COALESCE(new.notes, ''), new.keywords);
END;

-- On DELETE: remove the asset's FTS entries using the special 'delete' command.
CREATE TRIGGER assets_fts_before_delete BEFORE DELETE ON assets BEGIN
    INSERT INTO assets_fts(assets_fts, rowid, name, notes, keywords)
    VALUES('delete', old.rowid, old.name, COALESCE(old.notes, ''), old.keywords);
END;

-- On UPDATE of searchable columns: re-index by deleting old and inserting new.
CREATE TRIGGER assets_fts_after_update AFTER UPDATE OF name, notes, keywords ON assets BEGIN
    INSERT INTO assets_fts(assets_fts, rowid, name, notes, keywords)
    VALUES('delete', old.rowid, old.name, COALESCE(old.notes, ''), old.keywords);
    INSERT INTO assets_fts(rowid, name, notes, keywords)
    VALUES (new.rowid, new.name, COALESCE(new.notes, ''), new.keywords);
END;
