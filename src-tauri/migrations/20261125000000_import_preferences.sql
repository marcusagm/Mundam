CREATE TABLE import_preferences (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    catalog_id TEXT NOT NULL,
    default_folder_id TEXT REFERENCES folders(id) ON DELETE SET NULL
);

INSERT INTO import_preferences (singleton, catalog_id) VALUES (1, lower(hex(randomblob(16))));
