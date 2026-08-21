ALTER TABLE subject_item ADD COLUMN material_tags TEXT NOT NULL DEFAULT '[]';
ALTER TABLE subject_item ADD COLUMN interest_tags TEXT NOT NULL DEFAULT '[]';

UPDATE subject_item SET
  material_tags = CASE category
    WHEN 'rhyme' THEN '["movement_space"]'
    WHEN 'counting' THEN '["household_objects"]'
    WHEN 'quantity' THEN '["household_objects"]'
    WHEN 'shape' THEN '["household_objects"]'
    ELSE '["toys_blocks"]'
  END,
  interest_tags = CASE category
    WHEN 'rhyme' THEN '["music","movement"]'
    WHEN 'counting' THEN '["building"]'
    WHEN 'quantity' THEN '["food"]'
    WHEN 'shape' THEN '["outdoors","building"]'
    ELSE '["animals"]'
  END
WHERE review_status = 'published';

UPDATE app_meta SET value = '8' WHERE key = 'schema_version';
