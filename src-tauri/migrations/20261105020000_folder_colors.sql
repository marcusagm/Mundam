ALTER TABLE folders ADD COLUMN color TEXT;
ALTER TABLE folders ADD COLUMN inherits_color INTEGER NOT NULL DEFAULT 1 CHECK (inherits_color IN (0, 1));
