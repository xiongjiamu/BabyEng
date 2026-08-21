//! M8 学习日报（7.3）、打卡日历与勋章（7.1）：我的页数据

use axum::extract::{Extension, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use chrono::{Duration, Utc};
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;

use crate::auth::{self, AuthUser};
use crate::error::AppResult;
use crate::logic;
use crate::state::SharedState;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/api/report/today", get(report_today))
        .route("/api/report/activity-week", get(activity_week))
        .route(
            "/api/report/activity-observations",
            get(activity_observations),
        )
        .route("/api/report/calendar", get(calendar))
        .route("/api/achievements", get(achievements))
        .route("/api/report/recordings-today", get(recordings_today))
}

/// 近 7 天亲子活动观察。只汇总照护者记录，不推断能力或发展水平。
async fn activity_week(
    State(state): State<SharedState>,
    Extension(user): Extension<AuthUser>,
    Query(q): Query<CQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let child_id = auth::resolve_child(&state.pool, &user, q.child_id.as_deref()).await?;
    let since = (Utc::now() - Duration::days(7)).to_rfc3339();
    let rows = sqlx::query(
        "SELECT lr.target_id, lr.mother_mark, lr.recorded_at, s.title, s.image_emoji, s.subject, s.category \
         FROM learning_record lr JOIN subject_item s ON s.id=lr.target_id \
         WHERE lr.child_id=? AND lr.target_type='subject_item' AND lr.action='observe' \
           AND lr.mother_mark IN ('observed_independent','observed_with_help','not_interested') \
           AND lr.recorded_at>=? ORDER BY lr.recorded_at DESC",
    )
    .bind(&child_id)
    .bind(&since)
    .fetch_all(&state.pool)
    .await?;

    let mut independent = 0_i64;
    let mut with_help = 0_i64;
    let mut not_interested = 0_i64;
    let mut active_days = std::collections::HashSet::new();
    let mut category_counts = std::collections::HashMap::<String, i64>::new();
    let mut recent = Vec::new();
    for row in rows {
        let mark: String = row.try_get("mother_mark")?;
        let recorded_at: String = row.try_get("recorded_at")?;
        active_days.insert(recorded_at.chars().take(10).collect::<String>());
        match mark.as_str() {
            "observed_independent" => independent += 1,
            "observed_with_help" => with_help += 1,
            "not_interested" => not_interested += 1,
            _ => continue,
        }
        if mark != "not_interested" {
            *category_counts
                .entry(row.try_get::<String, _>("category")?)
                .or_default() += 1;
        }
        if recent.len() < 10 {
            recent.push(json!({
                "id": row.try_get::<String, _>("target_id")?,
                "title": row.try_get::<String, _>("title")?,
                "image_emoji": row.try_get::<String, _>("image_emoji")?,
                "subject": row.try_get::<String, _>("subject")?,
                "category": row.try_get::<String, _>("category")?,
                "mark": mark,
                "recorded_at": recorded_at,
            }));
        }
    }
    let top_category = category_counts
        .into_iter()
        .max_by(|left, right| left.1.cmp(&right.1).then_with(|| right.0.cmp(&left.0)))
        .map(|(category, count)| json!({ "category": category, "count": count }));

    Ok(Json(json!({
        "days": 7,
        "active_days": active_days.len(),
        "activities_done": independent + with_help,
        "independent": independent,
        "with_help": with_help,
        "not_interested": not_interested,
        "top_category": top_category,
        "recent": recent,
    })))
}

#[derive(Deserialize)]
struct CQuery {
    child_id: Option<String>,
}

#[derive(Deserialize)]
struct ActivityObservationQuery {
    child_id: Option<String>,
    days: Option<i64>,
}

/// 导出可供家庭复核的结构化观察证据；数据仍只从当前账号所属孩子读取。
async fn activity_observations(
    State(state): State<SharedState>,
    Extension(user): Extension<AuthUser>,
    Query(q): Query<ActivityObservationQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let child_id = auth::resolve_child(&state.pool, &user, q.child_id.as_deref()).await?;
    let days = observation_days(q.days)?;
    let since = (Utc::now() - Duration::days(days)).to_rfc3339();
    let rows = sqlx::query(
        "SELECT lr.target_id, lr.mother_mark, lr.recorded_at, s.title, s.image_emoji, \
                s.subject, s.category, s.scene, s.observe_for \
         FROM learning_record lr JOIN subject_item s ON s.id=lr.target_id \
         WHERE lr.child_id=? AND lr.target_type='subject_item' AND lr.action='observe' \
           AND lr.mother_mark IN ('observed_independent','observed_with_help','not_interested') \
           AND lr.recorded_at>=? ORDER BY lr.recorded_at DESC LIMIT 1000",
    )
    .bind(&child_id)
    .bind(&since)
    .fetch_all(&state.pool)
    .await?;

    let mut observations = Vec::with_capacity(rows.len());
    let mut summaries = std::collections::BTreeMap::<String, ActivityObservationSummary>::new();
    for row in rows {
        let id: String = row.try_get("target_id")?;
        let mark: String = row.try_get("mother_mark")?;
        let summary = summaries
            .entry(id.clone())
            .or_insert(ActivityObservationSummary {
                title: row.try_get("title")?,
                subject: row.try_get("subject")?,
                category: row.try_get("category")?,
                independent: 0,
                with_help: 0,
                not_interested: 0,
            });
        summary.add(&mark);
        observations.push(json!({
            "activity_id": id,
            "title": row.try_get::<String, _>("title")?,
            "image_emoji": row.try_get::<String, _>("image_emoji")?,
            "subject": row.try_get::<String, _>("subject")?,
            "category": row.try_get::<String, _>("category")?,
            "scene": row.try_get::<String, _>("scene")?,
            "observe_for": row.try_get::<String, _>("observe_for")?,
            "mark": mark,
            "recorded_at": row.try_get::<String, _>("recorded_at")?,
        }));
    }
    let activity_summary = summaries
        .into_iter()
        .map(|(id, summary)| summary.into_json(id))
        .collect::<Vec<_>>();

    Ok(Json(json!({
        "format": "babyeng-activity-observations-v1",
        "exported_at": Utc::now().to_rfc3339(),
        "child_id": child_id,
        "days": days,
        "observation_count": observations.len(),
        "activity_summary": activity_summary,
        "observations": observations,
        "notice": "家庭日常观察记录，不代表能力测评或发展诊断。",
    })))
}

fn observation_days(days: Option<i64>) -> AppResult<i64> {
    let days = days.unwrap_or(30);
    if ![7, 30, 90].contains(&days) {
        return Err(crate::error::AppError::BadRequest(
            "days 仅支持 7、30 或 90".into(),
        ));
    }
    Ok(days)
}

struct ActivityObservationSummary {
    title: String,
    subject: String,
    category: String,
    independent: i64,
    with_help: i64,
    not_interested: i64,
}

impl ActivityObservationSummary {
    fn add(&mut self, mark: &str) {
        match mark {
            "observed_independent" => self.independent += 1,
            "observed_with_help" => self.with_help += 1,
            "not_interested" => self.not_interested += 1,
            _ => {}
        }
    }

    fn into_json(self, id: String) -> serde_json::Value {
        json!({
            "activity_id": id,
            "title": self.title,
            "subject": self.subject,
            "category": self.category,
            "independent": self.independent,
            "with_help": self.with_help,
            "not_interested": self.not_interested,
            "total": self.independent + self.with_help + self.not_interested,
        })
    }
}

/// 当日日报（PRD 7.3：轻量复盘，一屏读完，不堆数据）
async fn report_today(
    State(state): State<SharedState>,
    Extension(user): Extension<AuthUser>,
    Query(q): Query<CQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let pool = &state.pool;
    let child_id = auth::resolve_child(pool, &user, q.child_id.as_deref()).await?;
    let s = logic::today_summary(pool, &child_id).await?;

    // 今日新学词
    let today = chrono::Local::now()
        .date_naive()
        .format("%Y-%m-%d")
        .to_string();
    let new_words = sqlx::query(
        "SELECT DISTINCT w.id, w.en, w.phonetic FROM learning_record lr \
         JOIN word w ON w.id = lr.target_id \
         WHERE lr.child_id=? AND lr.target_type='word' AND lr.action IN ('learn','ask') AND substr(lr.recorded_at,1,10)=? \
         ORDER BY lr.recorded_at DESC LIMIT 10",
    )
    .bind(&child_id)
    .bind(&today)
    .fetch_all(pool)
    .await?;
    let mut words = Vec::new();
    for r in &new_words {
        words.push(json!({
            "id": r.try_get::<String, _>("id")?,
            "en": r.try_get::<String, _>("en")?,
            "phonetic": r.try_get::<Option<String>, _>("phonetic")?,
        }));
    }

    // 明日复习推荐：掌握度最低的前 2 个已学词（对应原型「明天先复习」）
    let tomorrow_review = sqlx::query(
        "SELECT p.target_id, p.mastery, w.en FROM progress p JOIN word w ON w.id=p.target_id \
         WHERE p.child_id=? AND p.target_type='word' AND p.mastery < 0.6 ORDER BY p.mastery ASC LIMIT 2",
    )
    .bind(&child_id)
    .fetch_all(pool)
    .await?;
    let mut review = Vec::new();
    for r in &tomorrow_review {
        review.push(json!({
            "id": r.try_get::<String, _>("target_id")?,
            "en": r.try_get::<String, _>("en")?,
            "mastery": r.try_get::<f64, _>("mastery")?,
        }));
    }

    // 亲子学习时长（录音总时长，纯音频不计入屏幕时间）
    let rec_ms: i64 =
        sqlx::query_scalar("SELECT COALESCE(rec_ms,0) FROM child_daily WHERE child_id=? AND day=?")
            .bind(&child_id)
            .bind(&today)
            .fetch_optional(pool)
            .await?
            .unwrap_or(0);

    Ok(Json(json!({
        "date": today,
        "child_id": child_id,
        "learned_today": s.learned_today,
        "rec_today": s.rec_today,
        "parent_time_min": (rec_ms as f64 / 60000.0 * 10.0).round() / 10.0,
        "screen_sec_today": s.screen_sec_today,
        "streak": s.streak,
        "new_words": words,
        "tomorrow_review": review,
    })))
}

/// 打卡日历（成就 Tab）
async fn calendar(
    State(state): State<SharedState>,
    Extension(user): Extension<AuthUser>,
    Query(q): Query<CQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let pool = &state.pool;
    let child_id = auth::resolve_child(pool, &user, q.child_id.as_deref()).await?;
    let cal = logic::month_calendar(pool, &child_id).await?;
    let s = logic::today_summary(pool, &child_id).await?;
    Ok(Json(json!({
        "calendar": cal,
        "streak": s.streak,
        "freeze_used": s.freeze_used,
        "freeze_left": s.freeze_left,
    })))
}

/// 勋章墙（7.1：场景学完 / 连续 7 天 / 跟读 50 次 / 100 词）
async fn achievements(
    State(state): State<SharedState>,
    Extension(user): Extension<AuthUser>,
    Query(q): Query<CQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let pool = &state.pool;
    let child_id = auth::resolve_child(pool, &user, q.child_id.as_deref()).await?;
    let unlocked = sqlx::query(
        "SELECT type, key, unlocked_at FROM achievement WHERE child_id=? ORDER BY unlocked_at",
    )
    .bind(&child_id)
    .fetch_all(pool)
    .await?;

    let medal_defs = vec![
        ("scene_item_done", "餐具学完", "🍽"),
        ("streak_7", "连续 7 天", "🔥"),
        ("rec_50", "跟读 50 次", "🎤"),
        ("scene_person_done", "人物学完", "👪"),
        ("streak_30", "连续 30 天", "🌙"),
        ("stars_100", "100 个词", "💯"),
    ];
    let unlocked_keys: std::collections::HashSet<String> = unlocked
        .iter()
        .filter_map(|r| r.try_get::<String, _>("key").ok())
        .collect();

    let mut medals = Vec::new();
    for (key, name, ico) in medal_defs {
        medals.push(json!({
            "key": key,
            "name": name,
            "icon": ico,
            "unlocked": unlocked_keys.contains(key),
        }));
    }
    Ok(Json(json!({ "medals": medals })))
}

/// 今日录音（我的页录音 Tab 按日分组）
async fn recordings_today(
    State(state): State<SharedState>,
    Extension(user): Extension<AuthUser>,
    Query(q): Query<CQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let pool = &state.pool;
    let child_id = auth::resolve_child(pool, &user, q.child_id.as_deref()).await?;
    let rows = sqlx::query(
        "SELECT r.id, r.target_type, r.target_id, r.duration_ms, r.favorited, r.created_at, w.en, s.en AS sent_en \
         FROM recording r LEFT JOIN word w ON w.id = r.target_id AND r.target_type='word' \
         LEFT JOIN sentence s ON s.id = r.target_id AND r.target_type='sentence' \
         WHERE r.child_id=? ORDER BY r.created_at DESC LIMIT 100",
    )
    .bind(&child_id)
    .fetch_all(pool)
    .await?;

    let mut out = Vec::new();
    for r in &rows {
        let en: String = r
            .try_get::<Option<String>, _>("en")?
            .or(r.try_get::<Option<String>, _>("sent_en")?)
            .unwrap_or_default();
        out.push(json!({
            "id": r.try_get::<String, _>("id")?,
            "target_type": r.try_get::<String, _>("target_type")?,
            "target_id": r.try_get::<String, _>("target_id")?,
            "en": en,
            "duration_ms": r.try_get::<i64, _>("duration_ms")?,
            "favorited": r.try_get::<i64, _>("favorited")? > 0,
            "created_at": r.try_get::<String, _>("created_at")?,
        }));
    }
    Ok(Json(json!({ "recordings": out })))
}

#[cfg(test)]
mod activity_observation_tests {
    use super::*;

    #[test]
    fn export_period_is_bounded_to_supported_windows() {
        assert_eq!(observation_days(None).unwrap(), 30);
        assert_eq!(observation_days(Some(7)).unwrap(), 7);
        assert_eq!(observation_days(Some(90)).unwrap(), 90);
        assert!(observation_days(Some(365)).is_err());
    }

    #[test]
    fn activity_summary_counts_all_observation_marks() {
        let mut summary = ActivityObservationSummary {
            title: "找圆形".into(),
            subject: "math".into(),
            category: "shape".into(),
            independent: 0,
            with_help: 0,
            not_interested: 0,
        };
        summary.add("observed_independent");
        summary.add("observed_with_help");
        summary.add("not_interested");
        summary.add("unexpected");
        let value = summary.into_json("math-shape-1".into());
        assert_eq!(value["total"], 3);
        assert_eq!(value["independent"], 1);
        assert_eq!(value["with_help"], 1);
        assert_eq!(value["not_interested"], 1);
    }
}
