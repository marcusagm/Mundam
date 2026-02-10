-- Add performance indices for filtering and sorting

-- Index for file format filtering (e.g., 'Video', 'GIF', etc.)
CREATE INDEX IF NOT EXISTS idx_images_format ON images(format);

-- Index for 'Date Added' sorting, common in the UI
CREATE INDEX IF NOT EXISTS idx_images_added_at ON images(added_at DESC);
