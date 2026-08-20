//! M3 录音（PRD 4.3 / 8.4）：上传、回放、收藏、过期清理、30 天保留

use axum::extract::{Extension, Multipart, Path, Query, State};
use axum::http::header;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use chrono::{Duration, Local};
use serde::Deserialize;
use serde_json::json;
use sqlx::sqlite::SqliteRow;
use sqlx::Row;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::auth::{self, AuthUser};
use crate::state::SharedState;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/api/recordings", get(list))
        .route("/api/recordings", post(upload))
        .route("/api/recordings/{id}", delete(remove))
        .route("/api/recordings/{id}/favorite", post(favorite))
        .route("/api/recordings/{id}/audio", get(audio))
        .route("/api/recordings/cleanup-expired", post(cleanup_expired))
}

#[derive(Deserialize)]
struct ListQuery {
    child_id: Option<String>,
}

async fn list(
    State(state): State<SharedState>,
    Extension(user): Extension<AuthUser>,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let pool = &state.pool;
    let child_id = auth::resolve_child(pool, &user, q.child_id.as_deref()).await?;
    let rows = fetch_recording_rows(pool, Some(&child_id)).await?;

    let mut out = Vec::new();
    for r in &rows {
        out.push(json!({
            "id": r.try_get::<String, _>("id")?,
            "child_id": r.try_get::<String, _>("child_id")?,
            "target_type": r.try_get::<String, _>("target_type")?,
            "target_id": r.try_get::<String, _>("target_id")?,
            "duration_ms": r.try_get::<i64, _>("duration_ms")?,
            "favorited": r.try_get::<i64, _>("favorited")? > 0,
            "created_at": r.try_get::<String, _>("created_at")?,
            "expires_at": r.try_get::<String, _>("expires_at")?,
        }));
    }
    Ok(Json(json!({ "recordings": out })))
}

async fn fetch_recording_rows(
    pool: &sqlx::SqlitePool,
    child_id: Option<&str>,
) -> Result<Vec<SqliteRow>, sqlx::Error> {
    if let Some(cid) = child_id {
        sqlx::query(
            "SELECT id, child_id, target_type, target_id, audio_path, duration_ms, favorited, created_at, expires_at \
             FROM recording WHERE child_id=? ORDER BY created_at DESC LIMIT 100",
        )
        .bind(cid)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query(
            "SELECT id, child_id, target_type, target_id, audio_path, duration_ms, favorited, created_at, expires_at \
             FROM recording ORDER BY created_at DESC LIMIT 100",
        )
        .fetch_all(pool)
        .await
    }
}

/// 上传录音（webm/opus 或 mp4/aac，落盘即存，PRD 9.2：不在前端长期堆积）
async fn upload(
    State(state): State<SharedState>,
    Extension(user): Extension<AuthUser>,
    mut multipart: Multipart,
) -> AppResult<Json<serde_json::Value>> {
    let mut audio: Option<Vec<u8>> = None;
    let mut ext = "webm".to_string();
    let mut child_id: Option<String> = None;
    let mut target_type = "word".to_string();
    let mut target_id = String::new();
    let mut duration_ms: i64 = 0;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::BadRequest("multipart 解析失败".into()))?
    {
        match field.name().unwrap_or("") {
            "audio" => {
                ext = field
                    .content_type()
                    .map(|t| {
                        if t.contains("mp4") || t.contains("aac") || t.contains("m4a") {
                            "m4a"
                        } else if t.contains("ogg") {
                            "ogg"
                        } else {
                            "webm"
                        }
                    })
                    .unwrap_or("webm")
                    .to_string();
                audio = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|_| AppError::BadRequest("读取音频失败".into()))?
                        .to_vec(),
                );
            }
            "child_id" => child_id = field.text().await.ok(),
            "target_type" => target_type = field.text().await.unwrap_or_else(|_| "word".into()),
            "target_id" => target_id = field.text().await.unwrap_or_default(),
            "duration_ms" => {
                duration_ms = field
                    .text()
                    .await
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0)
            }
            _ => {}
        }
    }

    let Some(audio) = audio else {
        return Err(AppError::BadRequest("缺少 audio 字段".into()));
    };
    let Some(child_id) = child_id else {
        return Err(AppError::BadRequest("缺少 child_id".into()));
    };
    auth::require_child(&state.pool, &user, &child_id).await?;
    if target_id.is_empty() {
        return Err(AppError::BadRequest("缺少 target_id".into()));
    }

    // PRD 4.3 / 5.4：< 0.5s 不入库、不计学习记录
    if duration_ms > 0 && duration_ms < 500 {
        return Err(AppError::BadRequest("录音过短".into()));
    }

    let rec_id = Uuid::new_v4().to_string();
    let dir = format!("{}/recordings", state.cfg.audio_dir);
    std::fs::create_dir_all(&dir)?;
    let file_name = format!("{}.{}", rec_id, ext);
    let path = format!("{}/{}", dir, file_name);
    std::fs::write(&path, &audio)?;

    let now = chrono::Utc::now();
    let expires = now + Duration::days(30); // 默认 30 天过期（8.4）
    sqlx::query(
        "INSERT INTO recording (id, child_id, target_type, target_id, audio_path, duration_ms, favorited, created_at, expires_at) \
         VALUES (?,?,?,?,?,?,0,?,?)",
    )
    .bind(&rec_id)
    .bind(&child_id)
    .bind(&target_type)
    .bind(&target_id)
    .bind(&path)
    .bind(duration_ms)
    .bind(now.to_rfc3339())
    .bind(expires.to_rfc3339())
    .execute(&state.pool)
    .await?;

    // 当日统计 + 打卡（7.1）
    crate::logic::bump_recording(&state.pool, &child_id, duration_ms).await?;

    Ok(Json(json!({
        "ok": true,
        "recording_id": rec_id,
        "duration_ms": duration_ms,
        "expires_at": expires.to_rfc3339(),
    })))
}

async fn favorite(
    State(state): State<SharedState>,
    Extension(user): Extension<AuthUser>,
    Path(id): Path<String>,
    Query(q): Query<serde_json::Value>,
) -> AppResult<Json<serde_json::Value>> {
    let fav = q.get("favorited").and_then(|v| v.as_bool()).unwrap_or(true);
    let rows = sqlx::query("UPDATE recording SET favorited=? WHERE id=? AND child_id IN (SELECT c.child_id FROM child c JOIN account_family a ON a.family_id=c.family_id WHERE a.username=?)")
        .bind(fav as i64)
        .bind(&id)
        .bind(&user.username)
        .execute(&state.pool)
        .await?;
    if rows.rows_affected() == 0 {
        return Err(AppError::NotFound("录音不存在".into()));
    }
    Ok(Json(json!({ "ok": true, "favorited": fav })))
}

async fn remove(
    State(state): State<SharedState>,
    Extension(user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let pool = &state.pool;
    let row = sqlx::query_as::<_, (String,)>("SELECT r.audio_path FROM recording r JOIN child c ON c.child_id=r.child_id JOIN account_family a ON a.family_id=c.family_id WHERE r.id=? AND a.username=?")
        .bind(&id)
        .bind(&user.username)
        .fetch_optional(pool)
        .await?;
    let Some((path,)) = row else {
        return Err(AppError::NotFound("录音不存在".into()));
    };
    let _ = std::fs::remove_file(&path);
    sqlx::query("DELETE FROM recording WHERE id=?")
        .bind(&id)
        .execute(pool)
        .await?;
    Ok(Json(json!({ "ok": true })))
}

/// 回放音频（文件流式返回）
async fn audio(
    State(state): State<SharedState>,
    Extension(user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> AppResult<axum::response::Response> {
    let row = sqlx::query_as::<_, (String,)>("SELECT r.audio_path FROM recording r JOIN child c ON c.child_id=r.child_id JOIN account_family a ON a.family_id=c.family_id WHERE r.id=? AND a.username=?")
        .bind(&id)
        .bind(&user.username)
        .fetch_optional(&state.pool)
        .await?;
    let Some((path,)) = row else {
        return Err(AppError::NotFound("录音不存在".into()));
    };
    let bytes = std::fs::read(&path)?;
    let mime = if path.ends_with(".m4a") || path.ends_with(".aac") {
        "audio/mp4"
    } else if path.ends_with(".ogg") {
        "audio/ogg"
    } else {
        "audio/webm"
    };
    let mut resp = axum::response::Response::new(axum::body::Body::from(bytes));
    resp.headers_mut()
        .insert(header::CONTENT_TYPE, header::HeaderValue::from_static(mime));
    Ok(resp)
}

/// 过期清理（30 天前非收藏，PRD 5.4 / 8.4）：设置页一键清理入口
async fn cleanup_expired(State(state): State<SharedState>, Extension(user): Extension<AuthUser>) -> AppResult<Json<serde_json::Value>> {
    let pool = &state.pool;
    let cutoff = (Local::now() - Duration::days(30))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT r.id, r.audio_path FROM recording r JOIN child c ON c.child_id=r.child_id JOIN account_family a ON a.family_id=c.family_id WHERE r.expires_at < ? AND r.favorited = 0 AND a.username=?",
    )
    .bind(&cutoff)
    .bind(&user.username)
    .fetch_all(pool)
    .await?;
    let rows_count = rows.len();
    let mut freed = 0usize;
    for (id, path) in rows {
        if let Ok(md) = std::fs::metadata(&path) {
            freed += md.len() as usize;
        }
        let _ = std::fs::remove_file(&path);
        sqlx::query("DELETE FROM recording WHERE id=?")
            .bind(&id)
            .execute(pool)
            .await?;
    }
    Ok(Json(json!({ "ok": true, "cleaned": rows_count, "freed_bytes": freed })))
}

#[cfg(test)]
mod tests {
    use super::fetch_recording_rows;

    #[tokio::test]
    async fn child_filter_treats_sql_metacharacters_as_data() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            "CREATE TABLE recording (id TEXT, child_id TEXT, target_type TEXT, target_id TEXT, audio_path TEXT, duration_ms INTEGER, favorited INTEGER, created_at TEXT, expires_at TEXT)",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO recording VALUES ('r1','child-1','word','word_cup','/tmp/r1',1000,0,'2026-01-01','2026-02-01')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let rows = fetch_recording_rows(&pool, Some("' OR 1=1 --"))
            .await
            .unwrap();
        assert!(rows.is_empty());
    }
}
