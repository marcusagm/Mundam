ALTER TABLE tags ADD COLUMN inherits_color BOOLEAN NOT NULL DEFAULT TRUE;
-- Preserve explicit colors in existing libraries; null colors adopt inheritance.
UPDATE tags SET inherits_color = FALSE WHERE color IS NOT NULL;
WITH RECURSIVE effective(id, color) AS (
    SELECT id, color FROM tags WHERE parent_id IS NULL
    UNION ALL
    SELECT child.id, CASE WHEN child.inherits_color THEN effective.color ELSE child.color END
    FROM tags child JOIN effective ON child.parent_id = effective.id
)
UPDATE tags SET color = (SELECT color FROM effective WHERE effective.id = tags.id)
WHERE inherits_color = TRUE;
