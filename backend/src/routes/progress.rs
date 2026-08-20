//! 学习记录提交 + 进度汇总 + 复习队列（PRD 8.3 / 8.6 / 4.2）

use axum::extract::{Extension, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;

use crate::error::AppResult;
use crate::auth::{self, AuthUser};
use crate::logic;
use crate::models::next_review_days;
use crate::state::SharedState;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/api/learning-records", post(record))
        .route("/api/progress/summary", get(summary))
        .route("/api/review/queue", get(review_queue))
        .route("/api/progress/word", get(word_progress))
}

#[derive(Deserialize)]
struct RecordBody {
    child_id: String,
    target_type: String,
    target_id: String,
    action: String, // learn / review / quiz / ask
    mother_mark: Option<String>,
    quiz_result: Option<String>,
}

async fn record(State(state): State<SharedState>, Extension(user): Extension<AuthUser>, Json(body): Json<RecordBody>) -> AppResult<Json<serde_json::Value>> {
    let pool = &state.pool;
    auth::require_child(pool, &user, &body.child_id).await?;
    if !["word", "sentence"].contains(&body.target_type.as_str()) {
        return Err(crate::error::AppError::BadRequest("target_type 非法".into()));
    }
    if !["learn", "review", "quiz", "ask"].contains(&body.action.as_str()) {
        return Err(crate::error::AppError::BadRequest("action 非法".into()));
    }
    if let Some(m) = &body.mother_mark {
        if !["got_it", "keep_trying"].contains(&m.as_str()) {
            return Err(crate::error::AppError::BadRequest("mother_mark 非法".into()));
        }
    }

    logic::record_learning(
        pool,
        &body.child_id,
        &body.target_type,
        &body.target_id,
        &body.action,
        body.mother_mark.as_deref(),
        body.quiz_result.as_deref(),
    )
    .await?;

    // 解锁成就（场景学完 / 连续 7 天 / 录音 50 次等）
    let unlocked = logic::check_achievements(pool, &body.child_id).await?;

    // 返回掌握度（用于前端展示「明天/3天后」文案）
    let mastery: Option<f64> = sqlx::query_scalar(
        "SELECT mastery FROM progress WHERE child_id=? AND target_type=? AND target_id=?",
    )
    .bind(&body.child_id)
    .bind(&body.target_type)
    .bind(&body.target_id)
    .fetch_optional(pool)
    .await?;

    Ok(Json(json!({
        "ok": true,
        "mastery": mastery,
        "next_review_days": mastery.and_then(next_review_days),
        "unlocked_achievements": unlocked,
    })))
}

#[derive(Deserialize)]
struct SummaryQuery {
    child_id: Option<String>,
}

async fn summary(State(state): State<SharedState>, Extension(user): Extension<AuthUser>, Query(q): Query<SummaryQuery>) -> AppResult<Json<serde_json::Value>> {
    let pool = &state.pool;
    let child_id = auth::resolve_child(pool, &user, q.child_id.as_deref()).await?;
    let s = logic::today_summary(pool, &child_id).await?;
    let total_words: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM word WHERE review_status='published'")
        .fetch_one(pool)
        .await?;
    let total_learned: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM progress WHERE child_id=? AND target_type='word'",
    )
    .bind(&child_id)
    .fetch_one(pool)
    .await?;

    Ok(Json(json!({
        "child_id": child_id,
        "learned_today": s.learned_today,
        "daily_goal": s.daily_goal,
        "rec_today": s.rec_today,
        "streak": s.streak,
        "freeze_used": s.freeze_used,
        "freeze_left": s.freeze_left,
        "screen_sec_today": s.screen_sec_today,
        "total_learned": total_learned,
        "total_words": total_words,
        "progress_ratio": if total_words > 0 { total_learned as f64 / total_words as f64 } else { 0.0 },
    })))
}

/// 复习队列：仅推送昨日及之前学过、掌握度低的词（4.2 / 8.6），按 next_review_at 排序
async fn review_queue(State(state): State<SharedState>, Extension(user): Extension<AuthUser>, Query(q): Query<SummaryQuery>) -> AppResult<Json<serde_json::Value>> {
    let pool = &state.pool;
    let child_id = auth::resolve_child(pool, &user, q.child_id.as_deref()).await?;
    let now = chrono::Utc::now().to_rfc3339();

    // 已学但掌握度 < 0.85，且 next_review_at <= now（到期）
    let rows = sqlx::query(
        "SELECT p.target_id, p.mastery, p.next_review_at, w.zh, w.en, w.phonetic, w.image_emoji \
         FROM progress p JOIN word w ON w.id = p.target_id \
         WHERE p.child_id=? AND p.target_type='word' AND p.mastery < 0.85 AND p.next_review_at <= ? \
         ORDER BY p.next_review_at ASC, p.mastery ASC LIMIT 10",
    )
    .bind(&child_id)
    .bind(&now)
    .fetch_all(pool)
    .await?;

    let mut queue = Vec::new();
    for r in &rows {
        queue.push(json!({
            "target_type": "word",
            "target_id": r.try_get::<String, _>("target_id")?,
            "zh": r.try_get::<String, _>("zh")?,
            "en": r.try_get::<String, _>("en")?,
            "phonetic": r.try_get::<Option<String>, _>("phonetic")?,
            "image_emoji": r.try_get::<String, _>("image_emoji")?,
            "mastery": r.try_get::<f64, _>("mastery")?,
            "review_label": logic::review_label(r.try_get::<f64, _>("mastery")?),
        }));
    }

    Ok(Json(json!({
        "child_id": child_id,
        "queue": queue,
        "count": queue.len(),
    })))
}

/// 单个词的学习进度（学习流里展示掌握度文案）
async fn word_progress(
    State(state): State<SharedState>,
    Extension(user): Extension<AuthUser>,
    Query(q): Query<ProgressQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let pool = &state.pool;
    auth::require_child(pool, &user, &q.child_id).await?;
    let row = sqlx::query_as::<_, (i64, i64, f64, Option<String>)>(
        "SELECT learn_count, review_count, mastery, next_review_at FROM progress WHERE child_id=? AND target_type='word' AND target_id=?",
    )
    .bind(&q.child_id)
    .bind(&q.target_id)
    .fetch_optional(pool)
    .await?;
    let Some((learn_count, review_count, mastery, _next)) = row else {
        return Ok(Json(json!({ "learned": false })));
    };
    Ok(Json(json!({
        "learned": true,
        "learn_count": learn_count,
        "review_count": review_count,
        "mastery": mastery,
        "review_label": logic::review_label(mastery),
    })))
}

#[derive(Deserialize)]
struct ProgressQuery {
    child_id: String,
    target_id: String,
}
