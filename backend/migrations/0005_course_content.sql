CREATE TABLE subject_item (
  id            TEXT PRIMARY KEY,
  subject       TEXT NOT NULL CHECK(subject IN ('chinese', 'math')),
  category      TEXT NOT NULL,
  title         TEXT NOT NULL,
  prompt        TEXT NOT NULL,
  answer        TEXT NOT NULL,
  image_emoji   TEXT NOT NULL DEFAULT '',
  level         INTEGER NOT NULL DEFAULT 1,
  review_status TEXT NOT NULL DEFAULT 'draft',
  updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);

CREATE INDEX idx_subject_item_subject_status ON subject_item(subject, review_status);
UPDATE app_meta SET value = '5' WHERE key = 'schema_version';
