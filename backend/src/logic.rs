//! 业务逻辑核心：学习记录、掌握度算法（8.6）、复习排期、打卡/Streak Freeze（7.1）

use chrono::{DateTime, Duration, Local, Utc};
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
use uuid::Uuid;

use crate::error::AppResult;
use crate::models::next_review_days;

/// 记录一次学习/复习/小测/问答，并更新 progress 与 child_daily（PRD 8.3 / 8.6）
pub async fn record_learning(
    pool: &SqlitePool,
    child_id: &str,
    target_type: &str,
    target_id: &str,
    action: &str,
    mother_mark: Option<&str>,
    quiz_result: Option<&str>,
) -> AppResult<()> {
    let now = Utc::now();
    let id = Uuid::new_v4().to_string();

    // 1. 写明细流水
    sqlx::query(
        "INSERT INTO learning_record (id, child_id, target_type, target_id, action, mother_mark, quiz_result, recorded_at) \
         VALUES (?,?,?,?,?,?,?,?)",
    )
    .bind(&id)
    .bind(child_id)
    .bind(target_type)
    .bind(target_id)
    .bind(action)
    .bind(mother_mark)
    .bind(quiz_result)
    .bind(now.to_rfc3339())
    .execute(pool)
    .await?;

    // 2. 更新 progress（upsert）
    let row =
        sqlx::query("SELECT * FROM progress WHERE child_id=? AND target_type=? AND target_id=?")
            .bind(child_id)
            .bind(target_type)
            .bind(target_id)
            .fetch_optional(pool)
            .await?;

    let (mut learn_count, mut review_count, marks_json, quiz_json): (i64, i64, String, String) =
        match row {
            Some(r) => (
                r.try_get("learn_count")?,
                r.try_get("review_count")?,
                r.try_get::<String, _>("last_mother_marks")?,
                r.try_get::<String, _>("last_quiz_results")?,
            ),
            None => (0, 0, "[]".into(), "[]".into()),
        };

    let mut marks: Vec<String> = serde_json::from_str(&marks_json).unwrap_or_default();
    let mut quizs: Vec<String> = serde_json::from_str(&quiz_json).unwrap_or_default();

    match action {
        "learn" | "ask" => learn_count += 1,
        "review" => review_count += 1,
        _ => {}
    }
    if let Some(m) = mother_mark {
        marks.push(m.to_string());
        if marks.len() > 3 {
            marks.remove(0);
        }
    }
    if let Some(q) = quiz_result {
        quizs.push(q.to_string());
        if quizs.len() > 3 {
            quizs.remove(0);
        }
    }

    let exposure = (learn_count + review_count).min(6) as f64 / 6.0;
    let recency = 1.0; // 刚记录，距 last_touched_at 0 天 → recency = 1
    let mastery = compute_mastery(&marks, &quizs, exposure, recency);

    let next_days = next_review_days(mastery);
    let next_review_at = next_days.map(|d| (now + Duration::days(d)).to_rfc3339());

    sqlx::query(
        "INSERT INTO progress (child_id, target_type, target_id, learn_count, review_count, last_mother_marks, last_quiz_results, last_touched_at, next_review_at, mastery) \
         VALUES (?,?,?,?,?,?,?,?,?,?) \
         ON CONFLICT(child_id, target_type, target_id) DO UPDATE SET \
           learn_count=excluded.learn_count, review_count=excluded.review_count, \
           last_mother_marks=excluded.last_mother_marks, last_quiz_results=excluded.last_quiz_results, \
           last_touched_at=excluded.last_touched_at, next_review_at=excluded.next_review_at, mastery=excluded.mastery",
    )
    .bind(child_id)
    .bind(target_type)
    .bind(target_id)
    .bind(learn_count)
    .bind(review_count)
    .bind(serde_json::to_string(&marks)?)
    .bind(serde_json::to_string(&quizs)?)
    .bind(now.to_rfc3339())
    .bind(&next_review_at)
    .bind(mastery)
    .execute(pool)
    .await?;

    // 3. 更新当日统计（打卡/日报）
    let local_day = now
        .with_timezone(&Local)
        .date_naive()
        .format("%Y-%m-%d")
        .to_string();
    match action {
        "learn" | "ask" => {
            sqlx::query(
                "INSERT INTO child_daily (child_id, day, learn_count) VALUES (?,?,1) \
                 ON CONFLICT(child_id, day) DO UPDATE SET learn_count = learn_count + 1",
            )
            .bind(child_id)
            .bind(&local_day)
            .execute(pool)
            .await?;
        }
        "review" | "observe" => {
            // review 也算当日学习动作（计入打卡），但不加 learn_count
            sqlx::query(
                "INSERT OR IGNORE INTO child_daily (child_id, day, learn_count) VALUES (?,?,0)",
            )
            .bind(child_id)
            .bind(&local_day)
            .execute(pool)
            .await?;
        }
        _ => {}
    }

    Ok(())
}

/// 掌握度算法（PRD 8.6 v0.4 修订：以母亲标记为主信号，删去不可靠的 similarity）
fn compute_mastery(marks: &[String], quizs: &[String], exposure: f64, recency: f64) -> f64 {
    let signal_m: Option<f64> = {
        let vals: Vec<f64> = marks
            .iter()
            .map(|m| match m.as_str() {
                "got_it" | "observed_independent" => 1.0,
                "observed_with_help" => 0.6,
                _ => 0.0,
            })
            .collect();
        if vals.is_empty() {
            None
        } else {
            Some(vals.iter().sum::<f64>() / vals.len() as f64)
        }
    };
    let quiz_acc: Option<f64> = {
        let vals: Vec<f64> = quizs
            .iter()
            .map(|q| if q == "correct" { 1.0 } else { 0.0 })
            .collect();
        if vals.is_empty() {
            None
        } else {
            Some(vals.iter().sum::<f64>() / vals.len() as f64)
        }
    };

    let m = match (signal_m, quiz_acc) {
        (Some(sm), _) => 0.55 * sm + 0.25 * exposure + 0.20 * recency,
        (None, Some(qa)) => 0.45 * qa + 0.35 * exposure + 0.20 * recency,
        (None, None) => 0.65 * exposure + 0.35 * recency,
    };
    // 夹到 0~1
    m.clamp(0.0, 1.0)
}

/// 录音入库后更新当日统计与打卡（PRD 8.4 / 7.1）
pub async fn bump_recording(pool: &SqlitePool, child_id: &str, duration_ms: i64) -> AppResult<()> {
    let local_day = Local::now().date_naive().format("%Y-%m-%d").to_string();
    sqlx::query(
        "INSERT INTO child_daily (child_id, day, rec_count, rec_ms) VALUES (?,?,1,?) \
         ON CONFLICT(child_id, day) DO UPDATE SET rec_count = rec_count + 1, rec_ms = rec_ms + excluded.rec_ms",
    )
    .bind(child_id)
    .bind(&local_day)
    .bind(duration_ms)
    .execute(pool)
    .await?;
    Ok(())
}

/// 累计幼儿实际看图时长。前端只在可见的幼儿学习页调用，纯音频模式不调用。
pub async fn bump_screen_time(pool: &SqlitePool, child_id: &str, seconds: i64) -> AppResult<i64> {
    let local_day = Local::now().date_naive().format("%Y-%m-%d").to_string();
    sqlx::query(
        "INSERT INTO child_daily (child_id, day, screen_sec) VALUES (?,?,?) \
         ON CONFLICT(child_id, day) DO UPDATE SET screen_sec = screen_sec + excluded.screen_sec",
    )
    .bind(child_id)
    .bind(&local_day)
    .bind(seconds)
    .execute(pool)
    .await?;

    let total = sqlx::query_scalar("SELECT screen_sec FROM child_daily WHERE child_id=? AND day=?")
        .bind(child_id)
        .bind(&local_day)
        .fetch_one(pool)
        .await?;
    Ok(total)
}

/// 今日小结（首页进度 + 打卡）
pub struct TodaySummary {
    pub learned_today: i64,
    pub daily_goal: i64,
    pub rec_today: i64,
    pub streak: i64,
    pub freeze_used: i64,
    pub freeze_left: i64,
    pub screen_sec_today: i64,
}

/// 连续打卡天数（含 Streak Freeze 使用，7.1）
pub async fn today_summary(pool: &SqlitePool, child_id: &str) -> AppResult<TodaySummary> {
    let today = Local::now().date_naive();
    let today_str = today.format("%Y-%m-%d").to_string();

    // 一次查出历史打卡数据（今天往前 400 天，足以覆盖任何现实连续打卡），
    // 避免 streak 循环逐日查库（N+1 查询，首页每次加载都会跑）
    let cutoff = (today - Duration::days(400)).format("%Y-%m-%d").to_string();
    let rows = sqlx::query(
        "SELECT day, learn_count, rec_count, screen_sec, frozen \
         FROM child_daily WHERE child_id=? AND day > ? AND day <= ?",
    )
    .bind(child_id)
    .bind(&cutoff)
    .bind(&today_str)
    .fetch_all(pool)
    .await?;

    let mut day_map: HashMap<String, (i64, i64, i64, i64)> = HashMap::new();
    for r in &rows {
        day_map.insert(
            r.try_get("day")?,
            (
                r.try_get::<i64, _>("learn_count")?,
                r.try_get::<i64, _>("rec_count")?,
                r.try_get::<i64, _>("screen_sec")?,
                r.try_get::<i64, _>("frozen")?,
            ),
        );
    }

    let today_data = day_map.get(&today_str).copied().unwrap_or((0, 0, 0, 0));
    let learned_today = today_data.0;
    let rec_today = today_data.1;
    let screen_sec_today = today_data.2;

    // 打卡：当天有学习或录音即算打卡日
    let checked_today = learned_today > 0 || rec_today > 0;

    // 连续天数：今天已打卡从今天起算，否则从昨天起算
    let mut streak: i64 = 0;
    let mut day = if checked_today {
        today
    } else {
        today - Duration::days(1)
    };
    let month = today.format("%Y-%m").to_string();
    let mut freeze_used_this_month: i64 = 0;
    let mut freeze_days_used: Vec<String> = Vec::new();

    // 本月的冻结记录（按 achievement 记）
    let freeze_rows = sqlx::query(
        "SELECT key FROM achievement WHERE child_id=? AND type='streak' AND key LIKE 'freeze_%'",
    )
    .bind(child_id)
    .fetch_all(pool)
    .await?;
    for r in freeze_rows {
        let key: String = r.try_get("key")?;
        if key.starts_with(&format!("freeze_{}", month)) {
            freeze_used_this_month += 1;
        }
        freeze_days_used.push(key.replace("freeze_", ""));
    }

    loop {
        let dstr = day.format("%Y-%m-%d").to_string();
        let (cnt, _frozen, _, _) = day_map.get(&dstr).copied().unwrap_or((0, 0, 0, 0));

        if cnt > 0 {
            streak += 1;
            day -= Duration::days(1);
        } else if freeze_days_used.contains(&dstr) {
            // 使用了打卡保护的当天：不算断签
            streak += 1;
            day -= Duration::days(1);
        } else {
            break;
        }
    }

    Ok(TodaySummary {
        learned_today,
        daily_goal: 5, // 默认目标 5 词（原型一致）
        rec_today,
        streak,
        freeze_used: freeze_used_this_month,
        freeze_left: (2 - freeze_used_this_month).max(0),
        screen_sec_today,
    })
}

/// 检查并解锁成就（PRD 7.1）：场景学完 / 连续 7 天 / 跟读 50 次 / 100 词
pub async fn check_achievements(pool: &SqlitePool, child_id: &str) -> AppResult<Vec<String>> {
    let mut unlocked = Vec::new();

    // 场景学完：某 category 的全部词都有 progress
    let scenes = sqlx::query("SELECT category, COUNT(*) total FROM word GROUP BY category")
        .fetch_all(pool)
        .await?;
    for r in scenes {
        let category: String = r.try_get("category")?;
        let total: i64 = r.try_get("total")?;
        let learned: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM word w WHERE w.category=? AND EXISTS \
             (SELECT 1 FROM progress p WHERE p.child_id=? AND p.target_type='word' AND p.target_id=w.id)",
        )
        .bind(&category)
        .bind(child_id)
        .fetch_one(pool)
        .await?;
        if learned >= total {
            let key = format!("scene_{}_done", category);
            unlocked.push(insert_achievement(pool, child_id, "medal", &key).await?);
        }
    }

    // 连续 7 天
    let s = today_summary(pool, child_id).await?;
    if s.streak >= 7 {
        unlocked.push(insert_achievement(pool, child_id, "streak", "streak_7").await?);
    }

    // 累计录音 50 次
    let rec_total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM recording WHERE child_id=?")
        .bind(child_id)
        .fetch_one(pool)
        .await?;
    if rec_total >= 50 {
        unlocked.push(insert_achievement(pool, child_id, "medal", "rec_50").await?);
    }

    // 星星 100（learn/ask + recording 计数）
    let stars: i64 = sqlx::query_scalar(
        "SELECT (SELECT COUNT(*) FROM learning_record WHERE child_id=? AND action IN ('learn','ask')) \
         + (SELECT COUNT(*) FROM recording WHERE child_id=?)",
    )
    .bind(child_id)
    .bind(child_id)
    .fetch_one(pool)
    .await?;
    if stars >= 100 {
        unlocked.push(insert_achievement(pool, child_id, "stars", "stars_100").await?);
    }

    Ok(unlocked)
}

async fn insert_achievement(
    pool: &SqlitePool,
    child_id: &str,
    r#type: &str,
    key: &str,
) -> AppResult<String> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let inserted = sqlx::query(
        "INSERT OR IGNORE INTO achievement (id, child_id, type, key, unlocked_at) VALUES (?,?,?,?,?)",
    )
    .bind(&id)
    .bind(child_id)
    .bind(r#type)
    .bind(key)
    .bind(&now)
    .execute(pool)
    .await?;
    if inserted.rows_affected() > 0 {
        Ok(key.to_string())
    } else {
        Ok(String::new())
    }
}

/// 本月打卡日历（我的页成就 Tab）
pub async fn month_calendar(pool: &SqlitePool, child_id: &str) -> AppResult<serde_json::Value> {
    let now = Local::now();
    let month = now.format("%Y-%m").to_string();
    let rows = sqlx::query("SELECT day, frozen FROM child_daily WHERE child_id=? AND day LIKE ?")
        .bind(child_id)
        .bind(format!("{}%", month))
        .fetch_all(pool)
        .await?;

    let mut days: Vec<serde_json::Value> = Vec::new();
    for r in rows {
        let day: String = r.try_get("day")?;
        let frozen: i64 = r.try_get("frozen")?;
        days.push(serde_json::json!({
            "day": day,
            "frozen": frozen > 0,
        }));
    }
    Ok(serde_json::json!({ "month": month, "days": days }))
}

/// 格式化复习文案：把掌握度翻译成「明天 / 3 天后」（PRD 8.6：不给母亲看数字）
pub fn review_label(mastery: f64) -> String {
    if mastery < 0.3 {
        "明天再来一次".into()
    } else if mastery < 0.6 {
        "3 天后".into()
    } else if mastery < 0.85 {
        "7 天后".into()
    } else {
        "已经很稳，不再主动推".into()
    }
}

#[allow(dead_code)]
pub fn _touch(pool: &SqlitePool) -> DateTime<Utc> {
    let _ = pool;
    Utc::now()
}

#[cfg(test)]
mod screen_time_tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn screen_time_increments_existing_daily_row() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query(
            "CREATE TABLE child_daily (child_id TEXT NOT NULL, day TEXT NOT NULL, learn_count INTEGER NOT NULL DEFAULT 0, rec_count INTEGER NOT NULL DEFAULT 0, rec_ms INTEGER NOT NULL DEFAULT 0, screen_sec INTEGER NOT NULL DEFAULT 0, frozen INTEGER NOT NULL DEFAULT 0, PRIMARY KEY(child_id, day))",
        )
        .execute(&pool)
        .await
        .unwrap();

        assert_eq!(bump_screen_time(&pool, "child-1", 15).await.unwrap(), 15);
        assert_eq!(bump_screen_time(&pool, "child-1", 12).await.unwrap(), 27);
    }
}
