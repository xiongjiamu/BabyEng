//! 配置加载（env 驱动，PRD 9.9：.env 管理 secret 与服务地址）

use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    /// 监听地址，默认 0.0.0.0:8080
    pub bind_addr: String,
    /// SQLite 文件路径
    pub database_url: String,
    /// 词条 seed 目录（JSON 文件）
    pub seed_dir: String,
    /// 录音与音频缓存根目录
    pub audio_dir: String,
    /// 推理服务地址（HTTP）
    pub tts_url: String,
    pub asr_url: String,
    pub llm_url: String,
    /// 主密钥（用于 API Key envelope 加密；开发默认随机，重启即失效）
    pub master_key: String,
    /// 音频转码可执行文件
    pub ffmpeg_bin: String,
    /// 静态前端目录（PWA 产物）
    pub static_dir: String,
    /// 推理服务调用超时（秒）
    pub svc_timeout_secs: u64,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            bind_addr: env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into()),
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://data/babyeng.db".into()),
            seed_dir: env::var("SEED_DIR").unwrap_or_else(|_| "data/seed".into()),
            audio_dir: env::var("AUDIO_DIR").unwrap_or_else(|_| "data/audio".into()),
            tts_url: env::var("TTS_URL").unwrap_or_else(|_| "http://127.0.0.1:8101".into()),
            asr_url: env::var("ASR_URL").unwrap_or_else(|_| "http://127.0.0.1:8102".into()),
            llm_url: env::var("LLM_URL").unwrap_or_else(|_| "http://127.0.0.1:8103".into()),
            master_key: env::var("MASTER_KEY").unwrap_or_else(|_| "dev-insecure-key".into()),
            ffmpeg_bin: env::var("FFMPEG_BIN").unwrap_or_else(|_| "ffmpeg".into()),
            static_dir: env::var("STATIC_DIR").unwrap_or_else(|_| "frontend/dist".into()),
            svc_timeout_secs: env::var("SVC_TIMEOUT_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
        }
    }
}
