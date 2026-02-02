-- Elleven Library Schema

CREATE TABLE IF NOT EXISTS folders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    parent_id INTEGER,
    path TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    is_root BOOLEAN DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (parent_id) REFERENCES folders(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS images (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    folder_id INTEGER NOT NULL,
    path TEXT NOT NULL UNIQUE,
    filename TEXT NOT NULL,
    width INTEGER,
    height INTEGER,
    size INTEGER,
    hash TEXT,
    thumbnail_path TEXT,
    format TEXT,
    rating INTEGER DEFAULT 0,
    notes TEXT,
    created_at DATETIME NOT NULL,
    modified_at DATETIME NOT NULL,
    added_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (folder_id) REFERENCES folders(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    parent_id INTEGER,
    color TEXT,
    order_index INTEGER DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (parent_id) REFERENCES tags(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS image_tags (
    image_id INTEGER NOT NULL,
    tag_id INTEGER NOT NULL,
    PRIMARY KEY (image_id, tag_id),
    FOREIGN KEY (image_id) REFERENCES images(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS smart_folders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    query_json TEXT NOT NULL, -- structured query object as JSON string
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_images_path ON images(path);
CREATE INDEX IF NOT EXISTS idx_images_folder ON images(folder_id);
CREATE INDEX IF NOT EXISTS idx_folders_parent ON folders(parent_id);
CREATE INDEX IF NOT EXISTS idx_folders_path ON folders(path);
CREATE INDEX IF NOT EXISTS idx_tags_name ON tags(name);

-- Performance Indices for Sorting
CREATE INDEX IF NOT EXISTS idx_images_rating_created ON images(rating DESC, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_images_modified ON images(modified_at DESC);
CREATE INDEX IF NOT EXISTS idx_images_size ON images(size DESC);
CREATE INDEX IF NOT EXISTS idx_images_created ON images(created_at DESC);

-- FTS5 Virtual Table for Fast Text Search
-- Uses 'trigram' tokenizer for efficient substring matching (LIKE %query%)
-- content='images' makes it an external content table (saves space)
CREATE VIRTUAL TABLE IF NOT EXISTS images_fts USING fts5(
    filename, 
    notes, 
    content='images', 
    content_rowid='id', 
    tokenize='trigram'
);

-- Triggers to keep FTS index in sync
CREATE TRIGGER IF NOT EXISTS images_ai AFTER INSERT ON images BEGIN
  INSERT INTO images_fts(rowid, filename, notes) VALUES (new.id, new.filename, new.notes);
END;

CREATE TRIGGER IF NOT EXISTS images_ad AFTER DELETE ON images BEGIN
  INSERT INTO images_fts(images_fts, rowid, filename, notes) VALUES('delete', old.id, old.filename, old.notes);
END;

CREATE TRIGGER IF NOT EXISTS images_au AFTER UPDATE ON images BEGIN
  INSERT INTO images_fts(images_fts, rowid, filename, notes) VALUES('delete', old.id, old.filename, old.notes);
  INSERT INTO images_fts(rowid, filename, notes) VALUES (new.id, new.filename, new.notes);
END;

CREATE TABLE IF NOT EXISTS app_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL, -- JSON Value
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

