//! M4 TTS 音频接口（PRD 4.4 / 9.4 / 9.10）
//! GET /api/tts/audio?text=&voice=&rate= → 缓存优先，未命中用 Flux TTS 合成并压 Opus
//! TTS 服务不可用 → 503（tts_unavailable），前端降级为「发音暂时不可用」不阻断（4.1.3）

use axum::extract::{Query, State};
use axum::http::header;
use axum::routing::get;
use axum::Router;
use serde::Deserialize;

use crate::error::AppResult;
use crate::state::SharedState;

pub fn router() -> Router<SharedState> {
    Router::new().route("/api/tts/audio", get(audio))
}

#[derive(Deserialize)]
struct TtsQuery {
    text: String,
    voice: Option<String>,
    rate: Option<f64>,
}

async fn audio(
    State(state): State<SharedState>,
    Query(q): Query<TtsQuery>,
) -> AppResult<axum::response::Response> {
    let text = q.text.trim().to_string();
    if text.is_empty() {
        return Err(crate::error::AppError::BadRequest("text 为空".into()));
    }
    let voice = q.voice.unwrap_or_else(|| "en_US-mike-medium".into());
    if !matches!(
        voice.as_str(),
        "en_US-mike-medium"
            | "en_US-amy-medium"
            | "en_US-ryan-medium"
            | "en_US-kristin-medium"
            | "en_US-hfc_female-medium"
            | "en_US-hfc_male-medium"
    ) {
        return Err(crate::error::AppError::BadRequest(
            "不支持的英语音色".into(),
        ));
    }
    let rate = q.rate.unwrap_or(0.8).clamp(0.5, 1.5);

    let (bytes, ext, _cached) = state.inference.tts_audio(&text, &voice, rate).await?;
    let mime = match ext.as_str() {
        "opus" => "audio/ogg",
        "mp3" => "audio/mpeg",
        _ => "audio/wav",
    };
    let mut resp = axum::response::Response::new(axum::body::Body::from(bytes));
    resp.headers_mut()
        .insert(header::CONTENT_TYPE, header::HeaderValue::from_static(mime));
    resp.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("public, max-age=86400"),
    );
    Ok(resp)
}
