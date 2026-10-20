-- Migration to add independent visual/crop score thresholds to duplicate_rule_sets
ALTER TABLE duplicate_rule_sets ADD COLUMN min_visual_score REAL NOT NULL DEFAULT 0.85;
ALTER TABLE duplicate_rule_sets ADD COLUMN min_crop_score REAL NOT NULL DEFAULT 0.85;
