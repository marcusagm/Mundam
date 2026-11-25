CREATE TABLE import_catalog_revisions (
    kind TEXT PRIMARY KEY CHECK (kind IN ('folders', 'tags')),
    revision INTEGER NOT NULL DEFAULT 0
) WITHOUT ROWID;

INSERT INTO import_catalog_revisions (kind) VALUES ('folders'), ('tags');

CREATE TRIGGER import_folders_insert AFTER INSERT ON folders BEGIN
    UPDATE import_catalog_revisions SET revision = revision + 1 WHERE kind = 'folders';
END;
CREATE TRIGGER import_folders_update AFTER UPDATE OF id, parent_id, name ON folders BEGIN
    UPDATE import_catalog_revisions SET revision = revision + 1 WHERE kind = 'folders';
END;
CREATE TRIGGER import_folders_delete AFTER DELETE ON folders BEGIN
    UPDATE import_catalog_revisions SET revision = revision + 1 WHERE kind = 'folders';
END;
CREATE TRIGGER import_tags_insert AFTER INSERT ON tags BEGIN
    UPDATE import_catalog_revisions SET revision = revision + 1 WHERE kind = 'tags';
END;
CREATE TRIGGER import_tags_update AFTER UPDATE OF id, parent_id, name, color, order_index ON tags BEGIN
    UPDATE import_catalog_revisions SET revision = revision + 1 WHERE kind = 'tags';
END;
CREATE TRIGGER import_tags_delete AFTER DELETE ON tags BEGIN
    UPDATE import_catalog_revisions SET revision = revision + 1 WHERE kind = 'tags';
END;
