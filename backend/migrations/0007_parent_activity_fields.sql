ALTER TABLE subject_item ADD COLUMN scene TEXT NOT NULL DEFAULT '';
ALTER TABLE subject_item ADD COLUMN materials TEXT NOT NULL DEFAULT '';
ALTER TABLE subject_item ADD COLUMN parent_script TEXT NOT NULL DEFAULT '';
ALTER TABLE subject_item ADD COLUMN child_action_a TEXT NOT NULL DEFAULT '';
ALTER TABLE subject_item ADD COLUMN child_action_b TEXT NOT NULL DEFAULT '';
ALTER TABLE subject_item ADD COLUMN observe_for TEXT NOT NULL DEFAULT '';
ALTER TABLE subject_item ADD COLUMN safety_note TEXT NOT NULL DEFAULT '';

UPDATE subject_item SET
  scene = CASE category WHEN 'rhyme' THEN 'bedtime' WHEN 'counting' THEN 'meal' WHEN 'quantity' THEN 'meal' WHEN 'shape' THEN 'outing' ELSE 'play' END,
  materials = CASE category WHEN 'rhyme' THEN '不需要材料，留出一点活动空间' WHEN 'counting' THEN '同类安全实物 1～4 个，例如积木或袜子' WHEN 'quantity' THEN '数量明显不同的两小堆安全实物' WHEN 'shape' THEN '家里相似形状的安全物品' ELSE '两个特征明显不同的安全实物' END,
  parent_script = prompt,
  child_action_a = CASE category WHEN 'rhyme' THEN '听妈妈念，跟着拍手或做动作。' WHEN 'counting' THEN '看妈妈逐个触碰物品并数数。' WHEN 'shape' THEN '跟妈妈摸一摸物品的边缘。' ELSE '跟着妈妈看一看、摸一摸。' END,
  child_action_b = CASE category WHEN 'rhyme' THEN '跟念一个词或模仿一个动作。' WHEN 'counting' THEN '自己逐个移动物品，妈妈同步数数。' WHEN 'quantity' THEN '指出符合要求的一边，答错也不纠缠。' WHEN 'shape' THEN '从两个物品中找出相同形状。' ELSE '按妈妈的要求指出其中一个。' END,
  observe_for = '观察宝宝是否愿意共同注意、模仿或动手尝试，不要求答对。',
  safety_note = CASE category WHEN 'rhyme' THEN '确认活动空间没有尖角和绊倒物。' ELSE '只使用不会误吞、没有尖角且适合宝宝抓握的物品。' END
WHERE review_status = 'published';

UPDATE app_meta SET value = '7' WHERE key = 'schema_version';
