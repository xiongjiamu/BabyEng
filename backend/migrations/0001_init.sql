-- BabyEng 数据库初始化（PRD 第 8 章 + 9.8）
-- SQLite / sqlx migrate

-- ---------- 家庭与孩子（8.5 / 9.8） ----------
CREATE TABLE family (
  family_id   TEXT PRIMARY KEY,
  mother_name TEXT NOT NULL DEFAULT '',
  settings    TEXT NOT NULL DEFAULT '{}',          -- json: 语速、屏幕时间上限、纯音频开关等家庭级设置
  created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);

CREATE TABLE child (
  child_id          TEXT PRIMARY KEY,
  family_id         TEXT NOT NULL REFERENCES family(family_id),
  child_name        TEXT NOT NULL DEFAULT '',
  child_birthdate   TEXT,                            -- YYYY-MM-DD，用于推导 A/B 分段
  age_band_override TEXT,                            -- 'A' / 'B' / NULL（母亲手动覆盖）
  level             INTEGER NOT NULL DEFAULT 1,      -- 1/2/3
  created_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);
CREATE INDEX idx_child_family ON child(family_id);

-- ---------- 词条 Word（8.1） ----------
CREATE TABLE word (
  id              TEXT PRIMARY KEY,                  -- 如 word_cup
  zh              TEXT NOT NULL,
  aliases         TEXT NOT NULL DEFAULT '[]',        -- json 数组，不可为空（匹配管线 L1 依赖）
  en              TEXT NOT NULL,
  pos             TEXT NOT NULL DEFAULT 'noun',
  phonetic        TEXT,                              -- 美式 IPA，如 /kʌp/；空表示「音标待确认」
  phonetic_source TEXT NOT NULL DEFAULT 'manual',    -- dict / g2p / manual，绝不含 llm（4.1.2）
  category        TEXT NOT NULL,                     -- item_furniture / person_family / number / emotion ...
  level           INTEGER NOT NULL DEFAULT 1,
  image_emoji     TEXT NOT NULL DEFAULT '',          -- 开发期占位（3.7：最终用自家实物照片）
  image_source    TEXT NOT NULL DEFAULT 'family',    -- family / cc0 / generated
  tts_audio_path  TEXT,
  tts_voice       TEXT,
  example_en      TEXT,
  example_zh      TEXT,
  mother_tip      TEXT,                              -- 母亲学习卡（M9）
  review_status   TEXT NOT NULL DEFAULT 'draft',     -- draft / audio_ok / published，未校音不下发
  created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);

-- ---------- 句子 Sentence（8.2） ----------
CREATE TABLE sentence (
  id               TEXT PRIMARY KEY,
  zh               TEXT NOT NULL,
  aliases          TEXT NOT NULL DEFAULT '[]',
  en               TEXT NOT NULL,
  phonetic         TEXT,
  phonetic_source  TEXT NOT NULL DEFAULT 'manual',
  scene            TEXT NOT NULL,                    -- morning / meal / play / bedtime / outing
  tts_audio_path   TEXT,
  tts_voice        TEXT,
  example_context  TEXT,                             -- 使用场景说明
  review_status    TEXT NOT NULL DEFAULT 'draft',
  created_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);

-- ---------- 别名展开表（8.9：L1 精确匹配的检索表，唯一索引） ----------
CREATE TABLE word_alias (
  alias   TEXT NOT NULL,
  word_id TEXT NOT NULL,
  PRIMARY KEY (alias, word_id)
);
CREATE INDEX idx_word_alias_word ON word_alias(word_id);

CREATE TABLE sentence_alias (
  alias       TEXT NOT NULL,
  sentence_id TEXT NOT NULL,
  PRIMARY KEY (alias, sentence_id)
);
CREATE INDEX idx_sentence_alias_sent ON sentence_alias(sentence_id);

-- ---------- 学习记录 LearningRecord（8.3，明细流水） ----------
CREATE TABLE learning_record (
  id           TEXT PRIMARY KEY,
  child_id     TEXT NOT NULL REFERENCES child(child_id),
  target_type  TEXT NOT NULL,                        -- word / sentence
  target_id    TEXT NOT NULL,
  action       TEXT NOT NULL,                        -- learn / review / quiz / ask
  mother_mark  TEXT,                                 -- got_it / keep_trying / null
  quiz_result  TEXT,                                 -- 仅 quiz：correct / wrong / skipped
  recorded_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);
CREATE INDEX idx_learnrec_child_time ON learning_record(child_id, recorded_at);

-- ---------- 录音 Recording（8.4） ----------
CREATE TABLE recording (
  id          TEXT PRIMARY KEY,
  child_id    TEXT NOT NULL REFERENCES child(child_id),
  target_type TEXT NOT NULL,
  target_id   TEXT NOT NULL,
  audio_path  TEXT NOT NULL,
  duration_ms INTEGER NOT NULL DEFAULT 0,
  similarity  REAL,                                  -- 保留可空，MVP/V1 不写入（4.3）
  favorited   INTEGER NOT NULL DEFAULT 0,
  created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
  expires_at  TEXT NOT NULL                          -- 默认 +30 天
);
CREATE INDEX idx_rec_child_time ON recording(child_id, created_at);
CREATE INDEX idx_rec_expires ON recording(expires_at);

-- ---------- 学习进度 Progress（8.6） ----------
CREATE TABLE progress (
  child_id         TEXT NOT NULL,
  target_type      TEXT NOT NULL,
  target_id        TEXT NOT NULL,
  learn_count      INTEGER NOT NULL DEFAULT 0,
  review_count     INTEGER NOT NULL DEFAULT 0,
  last_mother_marks  TEXT NOT NULL DEFAULT '[]',     -- json 最近 3 次
  last_quiz_results  TEXT NOT NULL DEFAULT '[]',     -- json 最近 3 次（仅 B 段）
  last_touched_at  TEXT,
  next_review_at   TEXT,
  mastery          REAL NOT NULL DEFAULT 0,
  PRIMARY KEY (child_id, target_type, target_id)
);
CREATE INDEX idx_progress_review ON progress(child_id, next_review_at);

-- ---------- 成就 Achievement（8.7） ----------
CREATE TABLE achievement (
  id          TEXT PRIMARY KEY,
  child_id    TEXT NOT NULL REFERENCES child(child_id),
  type        TEXT NOT NULL,                         -- medal / streak / stars
  key         TEXT NOT NULL,                         -- 如 scene_item_done / streak_7
  unlocked_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);
CREATE UNIQUE INDEX idx_achievement_unique ON achievement(child_id, key);

-- ---------- 未命中查询 UnmatchedQuery（8.8，词库扩充输入源） ----------
CREATE TABLE unmatched_query (
  id                TEXT PRIMARY KEY,
  family_id         TEXT NOT NULL REFERENCES family(family_id),
  raw_text          TEXT NOT NULL,
  normalized_text   TEXT NOT NULL,
  asr_confidence    REAL,
  llm_result        TEXT,                            -- json，若走到 L4
  hit_count         INTEGER NOT NULL DEFAULT 1,
  status            TEXT NOT NULL DEFAULT 'pending', -- pending / adopted / rejected
  last_seen_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);
CREATE UNIQUE INDEX idx_unmatched_norm ON unmatched_query(family_id, normalized_text);

-- ---------- 模型配置（9.8） ----------
CREATE TABLE model_config (
  config_id   TEXT PRIMARY KEY,
  family_id   TEXT NOT NULL REFERENCES family(family_id),
  type        TEXT NOT NULL,                         -- llm / tts / asr
  provider    TEXT NOT NULL,                         -- local / openai / ollama
  model_name  TEXT NOT NULL DEFAULT '',
  endpoint    TEXT,
  api_key_enc TEXT,                                  -- 服务器主密钥 envelope 加密（9.10）
  params      TEXT NOT NULL DEFAULT '{}',
  created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);

CREATE TABLE family_model_binding (
  family_id     TEXT PRIMARY KEY REFERENCES family(family_id),
  llm_config_id TEXT,
  tts_config_id TEXT,
  asr_config_id TEXT
);

-- ---------- 应用元数据 ----------
CREATE TABLE app_meta (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
INSERT INTO app_meta(key, value) VALUES ('schema_version', '1');
