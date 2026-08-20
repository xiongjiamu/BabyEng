//! 家庭数据导出与清空（PRD 11.4）。

use axum::extract::{Extension, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::{Row, SqlitePool};

use crate::error::{AppError, AppResult};
use crate::auth::{self, AuthUser};
use crate::state::SharedState;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/api/data/export", get(export_all))
        .route("/api/data/clear", post(clear_all))
}

async fn json_rows(pool: &SqlitePool, sql: &str, family_id: &str) -> AppResult<Value> {
    let raw: String = sqlx::query_scalar(sql).bind(family_id).fetch_one(pool).await?;
    Ok(serde_json::from_str(&raw)?)
}

async fn export_all(State(state): State<SharedState>, Extension(user): Extension<AuthUser>) -> AppResult<Json<Value>> {
    let pool = &state.pool;
    let family_id = auth::require_family_id(pool, &user).await?;
    let families = json_rows(pool, "SELECT COALESCE(json_group_array(json_object('family_id',family_id,'mother_name',mother_name,'settings',json(settings),'created_at',created_at)),'[]') FROM family WHERE family_id=?", &family_id).await?;
    let children = json_rows(pool, "SELECT COALESCE(json_group_array(json_object('child_id',child_id,'family_id',family_id,'child_name',child_name,'child_birthdate',child_birthdate,'age_band_override',age_band_override,'level',level,'created_at',created_at)),'[]') FROM child WHERE family_id=?", &family_id).await?;
    let learning_records = json_rows(pool, "SELECT COALESCE(json_group_array(json_object('id',lr.id,'child_id',lr.child_id,'target_type',lr.target_type,'target_id',lr.target_id,'action',lr.action,'mother_mark',lr.mother_mark,'quiz_result',lr.quiz_result,'recorded_at',lr.recorded_at)),'[]') FROM learning_record lr JOIN child c ON c.child_id=lr.child_id WHERE c.family_id=?", &family_id).await?;
    let progress = json_rows(pool, "SELECT COALESCE(json_group_array(json_object('child_id',p.child_id,'target_type',p.target_type,'target_id',p.target_id,'learn_count',p.learn_count,'review_count',p.review_count,'last_mother_marks',json(p.last_mother_marks),'last_quiz_results',json(p.last_quiz_results),'last_touched_at',p.last_touched_at,'next_review_at',p.next_review_at,'mastery',p.mastery)),'[]') FROM progress p JOIN child c ON c.child_id=p.child_id WHERE c.family_id=?", &family_id).await?;
    let achievements = json_rows(pool, "SELECT COALESCE(json_group_array(json_object('id',a.id,'child_id',a.child_id,'type',a.type,'key',a.key,'unlocked_at',a.unlocked_at)),'[]') FROM achievement a JOIN child c ON c.child_id=a.child_id WHERE c.family_id=?", &family_id).await?;
    let daily = json_rows(pool, "SELECT COALESCE(json_group_array(json_object('child_id',d.child_id,'day',d.day,'learn_count',d.learn_count,'rec_count',d.rec_count,'rec_ms',d.rec_ms,'screen_sec',d.screen_sec,'frozen',d.frozen)),'[]') FROM child_daily d JOIN child c ON c.child_id=d.child_id WHERE c.family_id=?", &family_id).await?;
    let unmatched = json_rows(pool, "SELECT COALESCE(json_group_array(json_object('id',id,'family_id',family_id,'raw_text',raw_text,'normalized_text',normalized_text,'asr_confidence',asr_confidence,'llm_result',llm_result,'hit_count',hit_count,'status',status,'last_seen_at',last_seen_at)),'[]') FROM unmatched_query WHERE family_id=?", &family_id).await?;
    let model_configs = json_rows(pool, "SELECT COALESCE(json_group_array(json_object('config_id',config_id,'family_id',family_id,'type',type,'provider',provider,'model_name',model_name,'endpoint',endpoint,'params',json(params),'created_at',created_at)),'[]') FROM model_config WHERE family_id=?", &family_id).await?;

    let rows = sqlx::query(
        "SELECT r.id, r.child_id, r.target_type, r.target_id, r.audio_path, r.duration_ms, r.favorited, r.created_at, r.expires_at FROM recording r JOIN child c ON c.child_id=r.child_id WHERE c.family_id=? ORDER BY r.created_at",
    )
    .bind(&family_id)
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
    Extension(user): Extension<AuthUser>,
    Json(body): Json<ClearBody>,
) -> AppResult<Json<Value>> {
    if body.confirmation != "DELETE_ALL_LEARNING_DATA" {
        return Err(AppError::BadRequest("清空确认文本不匹配".into()));
    }

    let family_id = auth::require_family_id(&state.pool, &user).await?;
    let recording_rows = sqlx::query("SELECT r.audio_path FROM recording r JOIN child c ON c.child_id=r.child_id WHERE c.family_id=?")
        .bind(&family_id)
        .fetch_all(&state.pool)
        .await?;
    let recording_count = recording_rows.len();
    let learning_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM learning_record lr JOIN child c ON c.child_id=lr.child_id WHERE c.family_id=?")
        .bind(&family_id)
        .fetch_one(&state.pool)
        .await?;

    let mut tx = state.pool.begin().await?;
    for statement in [
        "DELETE FROM recording WHERE child_id IN (SELECT child_id FROM child WHERE family_id=?)",
        "DELETE FROM learning_record WHERE child_id IN (SELECT child_id FROM child WHERE family_id=?)",
        "DELETE FROM progress WHERE child_id IN (SELECT child_id FROM child WHERE family_id=?)",
        "DELETE FROM achievement WHERE child_id IN (SELECT child_id FROM child WHERE family_id=?)",
        "DELETE FROM child_daily WHERE child_id IN (SELECT child_id FROM child WHERE family_id=?)",
        "DELETE FROM unmatched_query WHERE family_id=?",
    ] {
        sqlx::query(statement).bind(&family_id).execute(&mut *tx).await?;
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
