CREATE TABLE processing_tool_health (
    tool TEXT PRIMARY KEY CHECK (tool IN ('ffmpeg', 'ffprobe', 'assimp')),
    state TEXT NOT NULL DEFAULT 'unknown' CHECK (state IN ('unknown', 'available', 'unavailable')),
    revision INTEGER NOT NULL DEFAULT 0,
    checked_at INTEGER
);
INSERT INTO processing_tool_health(tool) VALUES ('ffmpeg'), ('ffprobe'), ('assimp');
