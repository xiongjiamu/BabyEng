-- auth.json 账号与家庭数据空间的稳定映射。密码不进入数据库。
CREATE TABLE account_family (
  username   TEXT PRIMARY KEY,
  family_id  TEXT UNIQUE REFERENCES family(family_id),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);

UPDATE app_meta SET value = '3' WHERE key = 'schema_version';
