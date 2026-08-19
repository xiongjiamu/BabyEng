//! 健康检查与就绪探针（PRD 9.10：/healthz、/readyz 供容器编排探活）

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::json;

use crate::state::SharedState;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/api/healthz", get(healthz))
        .route("/api/readyz", get(readyz))
}

async fn healthz() -> Json<serde_json::Value> {
    Json(json!({ "ok": true, "service": "babyeng-backend", "version": "0.4.0" }))
}

async fn readyz(State(state): State<SharedState>) -> Json<serde_json::Value> {
    // 推理服务就绪状态（PRD 5.4「正在启动」提示条依据）
    let ready = {
        let r = state.inference.ready.read().unwrap();
        (r.tts, r.asr, r.llm)
    };
    Json(json!({
        "ok": true,
        "services": {
            "tts": ready.0,
            "asr": ready.1,
            "llm": ready.2,
        }
    }))
}
