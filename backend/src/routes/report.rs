//! M8 学习日报（7.3）、打卡日历与勋章（7.1）：我的页数据

use axum::extract::{Extension, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;

use crate::error::AppResult;
use crate::auth::{self, AuthUser};
use crate::logic;
use crate::state::SharedState;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/api/report/today", get(report_today))
        .route("/api/report/calendar", get(calendar))
        .route("/api/achievements", get(achievements))
        .route("/api/report/recordings-today", get(recordings_today))
}

#[derive(Deserialize)]
struct CQuery {
    child_id: Option<String>,
}

/// 当日日报（PRD 7.3：轻量复盘，一屏读完，不堆数据）
async fn report_today(State(state): State<SharedState>, Extension(user): Extension<AuthUser>, Query(q): Query<CQuery>) -> AppResult<Json<serde_json::Value>> {
    let pool = &state.pool;
    let child_id = auth::resolve_child(pool, &user, q.child_id.as_deref()).await?;
    let s = logic::today_summary(pool, &child_id).await?;

    // 今日新学词
    let today = chrono::Local::now().date_naive().format("%Y-%m-%d").to_string();
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
    let rec_ms: i64 = sqlx::query_scalar(
        "SELECT COALESCE(rec_ms,0) FROM child_daily WHERE child_id=? AND day=?",
    )
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
async fn calendar(State(state): State<SharedState>, Extension(user): Extension<AuthUser>, Query(q): Query<CQuery>) -> AppResult<Json<serde_json::Value>> {
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
async fn achievements(State(state): State<SharedState>, Extension(user): Extension<AuthUser>, Query(q): Query<CQuery>) -> AppResult<Json<serde_json::Value>> {
    let pool = &state.pool;
    let child_id = auth::resolve_child(pool, &user, q.child_id.as_deref()).await?;
    let unlocked = sqlx::query("SELECT type, key, unlocked_at FROM achievement WHERE child_id=? ORDER BY unlocked_at")
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
async fn recordings_today(State(state): State<SharedState>, Extension(user): Extension<AuthUser>, Query(q): Query<CQuery>) -> AppResult<Json<serde_json::Value>> {
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
