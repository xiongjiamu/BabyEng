-- 每日统计表：打卡（7.1）、Streak Freeze、日报（7.3）、屏幕时间（11.3）
CREATE TABLE IF NOT EXISTS child_daily (
  child_id   TEXT NOT NULL,
  day        TEXT NOT NULL,                -- 本地日期 YYYY-MM-DD
  learn_count  INTEGER NOT NULL DEFAULT 0, -- 当日学习词数
  rec_count    INTEGER NOT NULL DEFAULT 0, -- 当日跟读录音次数
  rec_ms       INTEGER NOT NULL DEFAULT 0, -- 当日录音总毫秒（亲子学习时长）
  screen_sec   INTEGER NOT NULL DEFAULT 0, -- 当日幼儿屏幕时长（纯音频不计入）
  frozen       INTEGER NOT NULL DEFAULT 0, -- 当日是否使用打卡保护
  PRIMARY KEY (child_id, day)
);
CREATE INDEX IF NOT EXISTS idx_child_daily_child ON child_daily(child_id, day);
