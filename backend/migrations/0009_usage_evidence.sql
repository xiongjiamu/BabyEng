-- PRD 13：从本版本起记录最小化的事件级使用证据。
-- 不保存提问原文；原有 unmatched_query 继续单独承担补词用途。
CREATE TABLE ask_event (
  id             TEXT PRIMARY KEY,
  family_id      TEXT NOT NULL REFERENCES family(family_id),
  child_id       TEXT REFERENCES child(child_id),
  input_mode     TEXT NOT NULL,                    -- text / voice
  status         TEXT NOT NULL,                    -- hit / ambiguous / nomatch / asr_fail / tts_only_down
  target_type    TEXT,
  target_id      TEXT,
  latency_ms     INTEGER NOT NULL DEFAULT 0,       -- 后端完整请求处理时长，不等同真机首屏时延
  asked_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX idx_ask_event_family_time ON ask_event(family_id, asked_at);
CREATE INDEX idx_ask_event_child_time ON ask_event(child_id, asked_at);

ALTER TABLE recording ADD COLUMN ask_event_id TEXT REFERENCES ask_event(id);
CREATE INDEX idx_recording_ask_event ON recording(ask_event_id);

CREATE TABLE recording_attempt (
  id           TEXT PRIMARY KEY,
  child_id     TEXT NOT NULL REFERENCES child(child_id),
  duration_ms  INTEGER NOT NULL DEFAULT 0,
  accepted     INTEGER NOT NULL DEFAULT 0,
  recording_id TEXT REFERENCES recording(id),
  created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX idx_recording_attempt_child_time ON recording_attempt(child_id, created_at);
