-- Repair empty groups and stale membership left by earlier cascade-based removals.
DELETE FROM duplicate_groups
WHERE NOT EXISTS (SELECT 1 FROM duplicate_candidates WHERE group_id = duplicate_groups.id);

UPDATE duplicate_groups SET
    candidate_count = (SELECT COUNT(*) FROM duplicate_candidates WHERE group_id = duplicate_groups.id),
    canonical_asset_id = CASE
        WHEN EXISTS (SELECT 1 FROM duplicate_candidates
                     WHERE group_id = duplicate_groups.id AND asset_id = duplicate_groups.canonical_asset_id)
        THEN canonical_asset_id
        ELSE (SELECT asset_id FROM duplicate_candidates WHERE group_id = duplicate_groups.id
              ORDER BY score DESC, asset_id LIMIT 1)
    END;
