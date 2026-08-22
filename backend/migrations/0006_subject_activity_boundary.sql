-- 1～3 岁阶段以口语、动作和实物互动为主；识字不属于当前产品范围。
-- 保留既有内容供后台后续评审，但不再向家庭学习端下发。
UPDATE subject_item
SET review_status = 'draft', updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now')
WHERE subject = 'chinese' AND category = 'character';

UPDATE app_meta SET value = '6' WHERE key = 'schema_version';
