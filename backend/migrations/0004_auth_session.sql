-- 登录会话持久化。每次成功访问会把 expires_at 滑动刷新到 30 天后。
CREATE TABLE auth_session (
  token         TEXT PRIMARY KEY,
  username      TEXT NOT NULL,
  created_at    TEXT NOT NULL,
  last_seen_at  TEXT NOT NULL,
  expires_at    TEXT NOT NULL
);

CREATE INDEX idx_auth_session_expires ON auth_session(expires_at);
CREATE INDEX idx_auth_session_username ON auth_session(username);
