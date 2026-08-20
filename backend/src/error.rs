//! 统一错误类型：所有错误都映射为 JSON，前端按 code 分支降级（PRD 5.4 / 9.10）

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("数据库错误: {0}")]
    Db(#[from] sqlx::Error),

    #[error("JSON 序列化错误: {0}")]
    Json(#[from] serde_json::Error),

    #[error("请求参数错误: {0}")]
    BadRequest(String),

    #[error("资源不存在: {0}")]
    NotFound(String),

    #[error("未授权: {0}")]
    Unauthorized(String),

    #[error("TTS 服务不可用")]
    TtsUnavailable,

    #[error("ASR 服务不可用")]
    AsrUnavailable,

    #[error("推理服务错误: {0}")]
    Inference(String),

    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("内部错误: {0}")]
    Internal(String),
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            AppError::Db(_) => "db_error",
            AppError::Json(_) => "json_error",
            AppError::BadRequest(_) => "bad_request",
            AppError::NotFound(_) => "not_found",
            AppError::Unauthorized(_) => "unauthorized",
            AppError::TtsUnavailable => "tts_unavailable",
            AppError::AsrUnavailable => "asr_unavailable",
            AppError::Inference(_) => "inference_error",
            AppError::Io(_) => "io_error",
            AppError::Internal(_) => "internal_error",
        }
    }

    pub fn status(&self) -> StatusCode {
        match self {
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::TtsUnavailable | AppError::AsrUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();
        tracing::error!("api error: {:?}", self);
        (
            status,
            Json(json!({
                "ok": false,
                "code": self.code(),
                "message": self.to_string(),
            })),
        )
            .into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
