//! 家庭数据导出与清空（PRD 11.4）。

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::{Row, SqlitePool};

use crate::error::{AppError, AppResult};
use crate::state::SharedState;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/api/data/export", get(export_all))
        .route("/api/data/clear", post(clear_all))
}

async fn json_rows(pool: &SqlitePool, sql: &str) -> AppResult<Value> {
    let raw: String = sqlx::query_scalar(sql).fetch_one(pool).await?;
    Ok(serde_json::from_str(&raw)?)
}

async fn export_all(State(state): State<SharedState>) -> AppResult<Json<Value>> {
    let pool = &state.pool;
    let families = json_rows(pool, "SELECT COALESCE(json_group_array(json_object('family_id',family_id,'mother_name',mother_name,'settings',json(settings),'created_at',created_at)),'[]') FROM family").await?;
    let children = json_rows(pool, "SELECT COALESCE(json_group_array(json_object('child_id',child_id,'family_id',family_id,'child_name',child_name,'child_birthdate',child_birthdate,'age_band_override',age_band_override,'level',level,'created_at',created_at)),'[]') FROM child").await?;
    let learning_records = json_rows(pool, "SELECT COALESCE(json_group_array(json_object('id',id,'child_id',child_id,'target_type',target_type,'target_id',target_id,'action',action,'mother_mark',mother_mark,'quiz_result',quiz_result,'recorded_at',recorded_at)),'[]') FROM learning_record").await?;
    let progress = json_rows(pool, "SELECT COALESCE(json_group_array(json_object('child_id',child_id,'target_type',target_type,'target_id',target_id,'learn_count',learn_count,'review_count',review_count,'last_mother_marks',json(last_mother_marks),'last_quiz_results',json(last_quiz_results),'last_touched_at',last_touched_at,'next_review_at',next_review_at,'mastery',mastery)),'[]') FROM progress").await?;
    let achievements = json_rows(pool, "SELECT COALESCE(json_group_array(json_object('id',id,'child_id',child_id,'type',type,'key',key,'unlocked_at',unlocked_at)),'[]') FROM achievement").await?;
    let daily = json_rows(pool, "SELECT COALESCE(json_group_array(json_object('child_id',child_id,'day',day,'learn_count',learn_count,'rec_count',rec_count,'rec_ms',rec_ms,'screen_sec',screen_sec,'frozen',frozen)),'[]') FROM child_daily").await?;
    let unmatched = json_rows(pool, "SELECT COALESCE(json_group_array(json_object('id',id,'family_id',family_id,'raw_text',raw_text,'normalized_text',normalized_text,'asr_confidence',asr_confidence,'llm_result',llm_result,'hit_count',hit_count,'status',status,'last_seen_at',last_seen_at)),'[]') FROM unmatched_query").await?;
    let model_configs = json_rows(pool, "SELECT COALESCE(json_group_array(json_object('config_id',config_id,'family_id',family_id,'type',type,'provider',provider,'model_name',model_name,'endpoint',endpoint,'params',json(params),'created_at',created_at)),'[]') FROM model_config").await?;

    let rows = sqlx::query(
        "SELECT id, child_id, target_type, target_id, audio_path, duration_ms, favorited, created_at, expires_at FROM recording ORDER BY created_at",
    )
    .fetch_all(pool)
    .await?;
    let mut recordings = Vec::with_capacity(rows.len());
    for row in rows {
        let path: String = row.try_get("audio_path")?;
        let bytes = std::fs::read(&path)?;
        recordings.push(json!({
            "id": row.try_get::<String, _>("id")?,
            "child_id": row.try_get::<String, _>("child_id")?,
            "target_type": row.try_get::<String, _>("target_type")?,
            "target_id": row.try_get::<String, _>("target_id")?,
            "duration_ms": row.try_get::<i64, _>("duration_ms")?,
            "favorited": row.try_get::<i64, _>("favorited")? > 0,
            "created_at": row.try_get::<String, _>("created_at")?,
            "expires_at": row.try_get::<String, _>("expires_at")?,
            "audio_file": std::path::Path::new(&path).file_name().and_then(|v| v.to_str()),
            "audio_base64": encode_base64(&bytes),
        }));
    }

    Ok(Json(json!({
        "format": "babyeng-backup-v1",
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "family": families,
        "children": children,
        "learning_records": learning_records,
        "progress": progress,
        "recordings": recordings,
        "achievements": achievements,
        "daily": daily,
        "unmatched_queries": unmatched,
        "model_configs": model_configs,
    })))
}

#[derive(Deserialize)]
struct ClearBody {
    confirmation: String,
}

async fn clear_all(
    State(state): State<SharedState>,
    Json(body): Json<ClearBody>,
) -> AppResult<Json<Value>> {
    if body.confirmation != "DELETE_ALL_LEARNING_DATA" {
        return Err(AppError::BadRequest("清空确认文本不匹配".into()));
    }

    let recording_rows = sqlx::query("SELECT audio_path FROM recording")
        .fetch_all(&state.pool)
        .await?;
    let recording_count = recording_rows.len();
    let learning_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM learning_record")
        .fetch_one(&state.pool)
        .await?;

    let mut tx = state.pool.begin().await?;
    for statement in [
        "DELETE FROM recording",
        "DELETE FROM learning_record",
        "DELETE FROM progress",
        "DELETE FROM achievement",
        "DELETE FROM child_daily",
        "DELETE FROM unmatched_query",
    ] {
        sqlx::query(statement).execute(&mut *tx).await?;
    }
    tx.commit().await?;

    for row in recording_rows {
        let path: String = row.try_get("audio_path")?;
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => tracing::warn!(path, error = %e, "删除录音文件失败"),
        }
    }

    Ok(Json(json!({
        "ok": true,
        "recordings_deleted": recording_count,
        "learning_records_deleted": learning_count,
    })))
}

fn encode_base64(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let a = chunk[0];
        let b = chunk.get(1).copied().unwrap_or(0);
        let c = chunk.get(2).copied().unwrap_or(0);
        output.push(TABLE[(a >> 2) as usize] as char);
        output.push(TABLE[(((a & 0x03) << 4) | (b >> 4)) as usize] as char);
        output.push(if chunk.len() > 1 {
            TABLE[(((b & 0x0f) << 2) | (c >> 6)) as usize] as char
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            TABLE[(c & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    output
}

#[cfg(test)]
mod tests {
    use super::encode_base64;

    #[test]
    fn base64_vectors() {
        assert_eq!(encode_base64(b""), "");
        assert_eq!(encode_base64(b"f"), "Zg==");
        assert_eq!(encode_base64(b"fo"), "Zm8=");
        assert_eq!(encode_base64(b"foo"), "Zm9v");
    }
}
