//! 未命中查询（8.8）：词库扩充的输入源，按 hit_count 排序进入待补清单

use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;

use crate::error::AppResult;
use crate::state::SharedState;

pub fn router() -> Router<SharedState> {
    Router::new().route("/api/unmatched", get(list))
}

#[derive(Deserialize)]
struct UQuery {
    family_id: Option<String>,
    limit: Option<i64>,
}

async fn list(State(state): State<SharedState>, Query(q): Query<UQuery>) -> AppResult<Json<serde_json::Value>> {
    let pool = &state.pool;
    let family_id = match q.family_id {
        Some(f) if !f.is_empty() => f,
        _ => {
            let Some(f) = sqlx::query_scalar::<_, String>("SELECT family_id FROM family LIMIT 1")
                .fetch_optional(pool)
                .await? else {
                return Ok(Json(json!({ "unmatched": [] })));
            };
            f
        }
    };
    let limit = q.limit.unwrap_or(50);
    let rows = sqlx::query(
        "SELECT id, raw_text, normalized_text, asr_confidence, hit_count, status, last_seen_at \
         FROM unmatched_query WHERE family_id=? ORDER BY hit_count DESC, last_seen_at DESC LIMIT ?",
    )
    .bind(&family_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let mut out = Vec::new();
    for r in &rows {
        out.push(json!({
            "id": r.try_get::<String, _>("id")?,
            "raw_text": r.try_get::<String, _>("raw_text")?,
            "normalized_text": r.try_get::<String, _>("normalized_text")?,
            "asr_confidence": r.try_get::<Option<f64>, _>("asr_confidence")?,
            "hit_count": r.try_get::<i64, _>("hit_count")?,
            "status": r.try_get::<String, _>("status")?,
            "last_seen_at": r.try_get::<String, _>("last_seen_at")?,
        }));
    }
    Ok(Json(json!({ "unmatched": out, "count": out.len() })))
}
