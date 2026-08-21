//! M1 语音问答主链路（PRD 4.1 / 4.1.1~4.1.3 / 5.3 / 6.1）
//! 文本提问 / 语音提问（ASR）→ 匹配管线 L0~L2 → TTS → 结果卡
//! 彻底未命中：相近词推荐 + 文字输入 + 静默写入 unmatched_query（8.8）
//! ASR/TTS 不可用：分别降级（4.1.3 / 5.4），不整页报错

use axum::extract::{Extension, Multipart, State};
use axum::routing::post;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;
use std::time::Instant;
use uuid::Uuid;

use crate::auth::{self, AuthUser};
use crate::error::{AppError, AppResult};
use crate::matcher::Match;
use crate::models::{AskResponse, AskResult};
use crate::state::SharedState;
use crate::store;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/api/ask/text", post(ask_text))
        .route("/api/ask/voice", post(ask_voice))
        .route("/api/ask/confirm", post(ask_confirm))
}

#[derive(Deserialize)]
struct AskTextBody {
    text: String,
    child_id: Option<String>,
    /// ASR 置信度（语音链路透传，用于未命中表归组，8.8）
    asr_confidence: Option<f64>,
}

async fn ask_text(
    State(state): State<SharedState>,
    Extension(user): Extension<AuthUser>,
    Json(body): Json<AskTextBody>,
) -> AppResult<Json<AskResponse>> {
    let started = Instant::now();
    let text = body.text.trim().to_string();
    if text.is_empty() {
        return Err(AppError::BadRequest("文本为空".into()));
    }
    if let Some(child_id) = body.child_id.as_deref() {
        auth::require_child(&state.pool, &user, child_id).await?;
    }
    let family_id = auth::family_id(&state.pool, &user).await?;
    let mut resp = run_pipeline(
        &state,
        &text,
        body.asr_confidence,
        family_id.as_deref(),
        body.child_id.as_deref(),
    )
    .await;
    resp.event_id = record_ask_event(
        &state,
        family_id.as_deref(),
        body.child_id.as_deref(),
        "text",
        &resp,
        started.elapsed().as_millis(),
    )
    .await;
    Ok(Json(resp))
}

/// 语音提问：multipart 上传音频 → ffmpeg 转 16k wav → ASR → 匹配
async fn ask_voice(
    State(state): State<SharedState>,
    Extension(user): Extension<AuthUser>,
    mut multipart: Multipart,
) -> AppResult<Json<AskResponse>> {
    let started = Instant::now();
    let mut audio_bytes: Option<Vec<u8>> = None;
    let mut ext = "webm".to_string();
    let mut child_id: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::BadRequest("multipart 解析失败".into()))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
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
                let data = field
                    .bytes()
                    .await
                    .map_err(|_| AppError::BadRequest("读取音频失败".into()))?;
                audio_bytes = Some(data.to_vec());
            }
            "child_id" => {
                child_id = field.text().await.ok();
            }
            _ => {}
        }
    }

    let Some(audio) = audio_bytes else {
        return Err(AppError::BadRequest("缺少 audio 字段".into()));
    };
    if let Some(id) = child_id.as_deref() {
        auth::require_child(&state.pool, &user, id).await?;
    }
    let family_id = auth::family_id(&state.pool, &user).await?;

    // 1. 转码 16k 单声道 wav（9.10：Android webm/opus、iOS mp4/aac 都转）
    let wav = state.inference.to_wav_16k(audio, ext).await?;
    // 2. ASR（不可用 → 明确降级：asr_fail，前端给「没听清/打字」兜底）
    let outcome = match state.inference.asr_recognize(wav).await {
        Ok(o) => o,
        Err(AppError::AsrUnavailable) => {
            let mut response = AskResponse {
                status: "asr_fail".into(),
                result: None,
                candidates: vec![],
                recognized_text: None,
                normalized_text: None,
                unmatched_id: None,
                event_id: None,
                message: Some("识别服务暂时不可用，可以打字提问".into()),
            };
            response.event_id = record_ask_event(
                &state,
                family_id.as_deref(),
                child_id.as_deref(),
                "voice",
                &response,
                started.elapsed().as_millis(),
            )
            .await;
            return Ok(Json(response));
        }
        Err(e) => return Err(e),
    };

    let text = outcome.text.trim().to_string();
    if text.is_empty() {
        let mut response = AskResponse {
            status: "asr_fail".into(),
            result: None,
            candidates: vec![],
            recognized_text: Some(String::new()),
            normalized_text: None,
            unmatched_id: None,
            event_id: None,
            message: Some("没听清，再说一次".into()),
        };
        response.event_id = record_ask_event(
            &state,
            family_id.as_deref(),
            child_id.as_deref(),
            "voice",
            &response,
            started.elapsed().as_millis(),
        )
        .await;
        return Ok(Json(response));
    }

    let mut resp = run_pipeline(
        &state,
        &text,
        outcome.confidence,
        family_id.as_deref(),
        child_id.as_deref(),
    )
    .await;
    resp.event_id = record_ask_event(
        &state,
        family_id.as_deref(),
        child_id.as_deref(),
        "voice",
        &resp,
        started.elapsed().as_millis(),
    )
    .await;
    Ok(Json(resp))
}

/// 二选一确认（PRD 4.1：你是说 A 还是 B？）
#[derive(Deserialize)]
struct ConfirmBody {
    target_type: String,
    target_id: String,
    child_id: Option<String>,
    event_id: Option<String>,
}

async fn ask_confirm(
    State(state): State<SharedState>,
    Extension(user): Extension<AuthUser>,
    Json(body): Json<ConfirmBody>,
) -> AppResult<Json<serde_json::Value>> {
    let pool = &state.pool;
    if let Some(id) = body.child_id.as_deref() {
        auth::require_child(pool, &user, id).await?;
    }
    let result = match body.target_type.as_str() {
        "word" => {
            let w = store::get_word(pool, &body.target_id).await?;
            let Some(w) = w else {
                return Err(AppError::NotFound("词条不存在".into()));
            };
            AskResult {
                target_type: "word".into(),
                target_id: w.id.clone(),
                zh: w.zh,
                en: w.en,
                phonetic: w.phonetic,
                phonetic_source: Some(w.phonetic_source),
                category: Some(w.category),
                scene: None,
                example_en: w.example_en,
                example_zh: w.example_zh,
                example_context: None,
                mother_tip: w.mother_tip,
                image_emoji: Some(w.image_emoji),
                match_level: "L2-confirm".into(),
                tts_available: false,
                tts_url: None,
            }
        }
        "sentence" => {
            let s = store::get_sentence(pool, &body.target_id).await?;
            let Some(s) = s else {
                return Err(AppError::NotFound("句子不存在".into()));
            };
            AskResult {
                target_type: "sentence".into(),
                target_id: s.id.clone(),
                zh: s.zh,
                en: s.en,
                phonetic: s.phonetic,
                phonetic_source: Some(s.phonetic_source),
                category: None,
                scene: Some(s.scene),
                example_en: None,
                example_zh: None,
                example_context: s.example_context,
                mother_tip: None,
                image_emoji: None,
                match_level: "L2-confirm".into(),
                tts_available: false,
                tts_url: None,
            }
        }
        _ => return Err(AppError::BadRequest("target_type 非法".into())),
    };

    // 计一次 ask
    if let Some(cid) = &body.child_id {
        crate::logic::record_learning(
            pool,
            cid,
            &result.target_type,
            &result.target_id,
            "ask",
            None,
            None,
        )
        .await?;
    }

    let confirmed_event_id = if let Some(event_id) = body.event_id.as_deref() {
        let family_id = auth::require_family_id(pool, &user).await?;
        let updated = sqlx::query(
            "UPDATE ask_event SET status='hit', target_type=?, target_id=? WHERE id=? AND family_id=? AND status='ambiguous' AND (child_id IS NULL OR child_id=?)",
        )
        .bind(&result.target_type)
        .bind(&result.target_id)
        .bind(event_id)
        .bind(family_id)
        .bind(body.child_id.as_deref())
        .execute(pool)
        .await?;
        (updated.rows_affected() == 1).then(|| event_id.to_string())
    } else {
        None
    };

    Ok(Json(
        json!({ "ok": true, "result": result, "event_id": confirmed_event_id }),
    ))
}

async fn record_ask_event(
    state: &SharedState,
    family_id: Option<&str>,
    child_id: Option<&str>,
    input_mode: &str,
    response: &AskResponse,
    elapsed_ms: u128,
) -> Option<String> {
    let family_id = family_id?;
    let id = Uuid::new_v4().to_string();
    let target = response.result.as_ref();
    let latency_ms = i64::try_from(elapsed_ms).unwrap_or(i64::MAX);
    let inserted = sqlx::query(
        "INSERT INTO ask_event (id, family_id, child_id, input_mode, status, target_type, target_id, latency_ms, asked_at) VALUES (?,?,?,?,?,?,?,?,?)",
    )
    .bind(&id)
    .bind(family_id)
    .bind(child_id)
    .bind(input_mode)
    .bind(&response.status)
    .bind(target.map(|value| value.target_type.as_str()))
    .bind(target.map(|value| value.target_id.as_str()))
    .bind(latency_ms)
    .bind(chrono::Utc::now().to_rfc3339())
    .execute(&state.pool)
    .await;
    match inserted {
        Ok(_) => Some(id),
        Err(error) => {
            tracing::warn!(%error, "记录本地问答指标失败");
            None
        }
    }
}

/// 匹配管线主流程（L0 → L1 → L2 → 兜底）
async fn run_pipeline(
    state: &SharedState,
    raw: &str,
    asr_confidence: Option<f64>,
    family_id: Option<&str>,
    child_id: Option<&str>,
) -> AskResponse {
    let pool = &state.pool;
    let normalized = crate::normalize::normalize(raw);

    let matcher = state.matcher.read().await;
    let outcome = matcher.match_query(&normalized, raw);
    drop(matcher);

    let tts_ready = state.inference.tts_available();

    match outcome {
        Match::Hit(target, level) => {
            let mut result = target.to_ask_result();
            result.match_level = level.to_string();
            match store::enrich_ask_result(pool, result).await {
                Ok(r) => {
                    result = r;
                    // TTS 音频 URL（发音是否可用取决于 TTS 服务）
                    result.tts_available = tts_ready;
                    if tts_ready {
                        result.tts_url = Some(format!(
                            "/api/tts/audio?text={}&voice=en_US-mike-medium&rate=0.8",
                            percent_encoding::utf8_percent_encode(
                                &result.en,
                                percent_encoding::NON_ALPHANUMERIC
                            )
                        ));
                    }
                    // 记录 ask 学习动作
                    if let Some(cid) = child_id {
                        let _ = crate::logic::record_learning(
                            pool,
                            cid,
                            &result.target_type,
                            &result.target_id,
                            "ask",
                            None,
                            None,
                        )
                        .await;
                    }
                    AskResponse {
                        status: if tts_ready { "hit" } else { "tts_only_down" }.into(),
                        result: Some(result),
                        candidates: vec![],
                        recognized_text: Some(raw.to_string()),
                        normalized_text: Some(normalized.clone()),
                        unmatched_id: None,
                        event_id: None,
                        message: None,
                    }
                }
                Err(_) => AskResponse {
                    status: "nomatch".into(),
                    result: None,
                    candidates: vec![],
                    recognized_text: Some(raw.to_string()),
                    normalized_text: Some(normalized),
                    unmatched_id: None,
                    event_id: None,
                    message: Some("内部错误，请重试".into()),
                },
            }
        }
        Match::Ambiguous(cands) => {
            let mut candidates = Vec::new();
            for c in cands {
                let mut r = c.to_ask_result();
                r.match_level = "L2-ambiguous".into();
                if let Ok(enriched) = store::enrich_ask_result(pool, r).await {
                    let mut enriched = enriched;
                    enriched.tts_available = tts_ready;
                    candidates.push(enriched);
                }
            }
            AskResponse {
                status: "ambiguous".into(),
                result: None,
                candidates,
                recognized_text: Some(raw.to_string()),
                normalized_text: Some(normalized),
                unmatched_id: None,
                event_id: None,
                message: Some("你是说哪一个？".into()),
            }
        }
        Match::Miss => {
            // 彻底未命中：相近词推荐 + 文字输入 + 静默写入未命中表（8.8）
            let fam_id = family_id.unwrap_or_default().to_string();

            let unmatched_id = if fam_id.is_empty() {
                None
            } else {
                logic_unmatched::upsert_unmatched(pool, &fam_id, raw, &normalized, asr_confidence)
                    .await
                    .ok()
            };

            // 相近词推荐：同 category 优先
            let m = state.matcher.read().await;
            let suggested = m.suggest_similar(Some("general"), "", 3);
            drop(m);
            let mut candidates = Vec::new();
            for s in suggested {
                let r = s.to_ask_result();
                if let Ok(enriched) = store::enrich_ask_result(pool, r).await {
                    candidates.push(enriched);
                }
            }

            AskResponse {
                status: "nomatch".into(),
                result: None,
                candidates,
                recognized_text: Some(raw.to_string()),
                normalized_text: Some(normalized.clone()),
                unmatched_id,
                event_id: None,
                message: Some("这个词还没准备好，我记下了".into()),
            }
        }
    }
}

/// 未命中表写入（独立小模块，避免 logic.rs 依赖膨胀）
pub mod logic_unmatched {
    use sqlx::SqlitePool;
    use uuid::Uuid;

    pub async fn upsert_unmatched(
        pool: &SqlitePool,
        family_id: &str,
        raw: &str,
        normalized: &str,
        asr_confidence: Option<f64>,
    ) -> crate::error::AppResult<String> {
        let now = chrono::Utc::now().to_rfc3339();
        let id = Uuid::new_v4().to_string();
        // normalized_text 唯一索引：重复出现走 hit_count+1（8.8 / 8.9）
        let existing = sqlx::query_as::<_, (String,)>(
            "SELECT id FROM unmatched_query WHERE family_id=? AND normalized_text=?",
        )
        .bind(family_id)
        .bind(normalized)
        .fetch_optional(pool)
        .await?;

        match existing {
            Some((uid,)) => {
                sqlx::query(
                    "UPDATE unmatched_query SET hit_count = hit_count + 1, last_seen_at=?, raw_text=? WHERE id=?",
                )
                .bind(&now)
                .bind(raw)
                .bind(&uid)
                .execute(pool)
                .await?;
                Ok(uid)
            }
            None => {
                sqlx::query(
                    "INSERT INTO unmatched_query (id, family_id, raw_text, normalized_text, asr_confidence, hit_count, status, last_seen_at) \
                     VALUES (?,?,?,?,?,1,'pending',?)",
                )
                .bind(&id)
                .bind(family_id)
                .bind(raw)
                .bind(normalized)
                .bind(asr_confidence)
                .bind(&now)
                .execute(pool)
                .await?;
                Ok(id)
            }
        }
    }
}
