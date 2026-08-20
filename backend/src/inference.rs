//! 推理服务客户端（PRD 9.3：Rust 通过 HTTP 调用独立 Python 推理服务）
//! 任一服务不可用 → 返回 503 由上层降级（9.10 熔断：TTS 挂返回文字+提示，不整页报错）

use std::io::Read;
use std::path::Path;
use std::time::Duration;

use crate::config::Config;
use crate::error::{AppError, AppResult};

pub struct InferenceClients {
    pub tts_url: String,
    pub asr_url: String,
    pub llm_url: String,
    pub timeout: Duration,
    pub ffmpeg_bin: String,
    pub audio_dir: String,
    /// 服务就绪状态（启动探针缓存）
    pub ready: std::sync::RwLock<ReadyState>,
}

#[derive(Default)]
pub struct ReadyState {
    pub tts: bool,
    pub asr: bool,
    pub llm: bool,
}

impl InferenceClients {
    pub fn new(cfg: &Config) -> Self {
        std::fs::create_dir_all(&cfg.audio_dir).ok();
        Self {
            tts_url: cfg.tts_url.clone(),
            asr_url: cfg.asr_url.clone(),
            llm_url: cfg.llm_url.clone(),
            timeout: Duration::from_secs(cfg.svc_timeout_secs),
            ffmpeg_bin: cfg.ffmpeg_bin.clone(),
            audio_dir: cfg.audio_dir.clone(),
            ready: std::sync::RwLock::new(ReadyState::default()),
        }
    }

    /// 轮询推理服务就绪状态（/readyz）
    pub fn refresh_ready(&self) {
        let tts = self.check(&self.tts_url, "/readyz");
        let asr = self.check(&self.asr_url, "/readyz");
        let llm = self.check(&self.llm_url, "/readyz");
        if let Ok(mut r) = self.ready.write() {
            r.tts = tts;
            r.asr = asr;
            r.llm = llm;
        }
    }

    fn check(&self, base: &str, path: &str) -> bool {
        let url = format!("{}{}", base.trim_end_matches('/'), path);
        match ureq::agent().get(&url).timeout(self.timeout).call() {
            Ok(resp) => resp.status() == 200,
            Err(_) => false,
        }
    }

    // ---------- TTS ----------

    /// 合成文本 → wav 字节。若模型缺失/服务不可用 → Err(AppError::TtsUnavailable)
    pub async fn tts_synthesize(
        &self,
        text: String,
        voice: String,
        rate: f64,
    ) -> AppResult<Vec<u8>> {
        let url = format!(
            "{}/synthesize?text={}&voice={}&length_scale={:.2}",
            self.tts_url.trim_end_matches('/'),
            percent_encoding::utf8_percent_encode(&text, percent_encoding::NON_ALPHANUMERIC),
            voice,
            rate
        );
        let timeout = self.timeout;
        tokio::task::spawn_blocking(move || {
            let resp = ureq::agent()
                .get(&url)
                .timeout(timeout)
                .call()
                .map_err(classify_tts_err)?;
            if resp.status() != 200 {
                return Err(AppError::TtsUnavailable);
            }
            let mut buf = Vec::new();
            resp.into_reader()
                .read_to_end(&mut buf)
                .map_err(|_| AppError::TtsUnavailable)?;
            Ok(buf)
        })
        .await
        .map_err(|e| AppError::Internal(format!("tts task: {}", e)))?
    }

    /// 获取 TTS 音频：缓存优先（按 model+voice+text+rate hash，PRD 9.10）
    /// 返回 (字节, 扩展名, 是否缓存命中)
    pub async fn tts_audio(
        &self,
        text: &str,
        voice: &str,
        rate: f64,
    ) -> AppResult<(Vec<u8>, String, bool)> {
        let key = format!("{}|{}|{:.2}", voice, text, rate);
        let hash = cache_hash(&key);
        let cache_dir = format!("{}/tts_cache", self.audio_dir);
        std::fs::create_dir_all(&cache_dir).ok();

        // 尝试 opus（已压缩）→ wav
        let opus_path = format!("{}/{}.opus", cache_dir, hash);
        if Path::new(&opus_path).exists() {
            let bytes = std::fs::read(&opus_path)?;
            return Ok((bytes, "opus".into(), true));
        }
        let wav_path = format!("{}/{}.wav", cache_dir, hash);
        if Path::new(&wav_path).exists() {
            let bytes = std::fs::read(&wav_path)?;
            return Ok((bytes, "wav".into(), true));
        }

        // 未命中 → 实时合成（Piper 输出 wav）→ 压 Opus 减带宽（9.10），失败则存 wav
        let wav = self
            .tts_synthesize(text.to_string(), voice.to_string(), rate)
            .await?;
        let converted = self.to_opus(&wav, &wav_path, &opus_path);
        match converted {
            Some(opus_bytes) => Ok((opus_bytes, "opus".into(), false)),
            None => {
                std::fs::write(&wav_path, &wav).ok();
                Ok((wav, "wav".into(), false))
            }
        }
    }

    /// 浏览器录音（webm/opus 或 mp4/aac）→ 16k 单声道 wav（PRD 9.10 按平台分支转码）
    pub async fn to_wav_16k(&self, input_bytes: Vec<u8>, input_ext: String) -> AppResult<Vec<u8>> {
        let tmp_dir = format!("{}/tmp", self.audio_dir);
        let ffmpeg = self.ffmpeg_bin.clone();
        tokio::task::spawn_blocking(move || {
            std::fs::create_dir_all(&tmp_dir).ok();
            let in_path = format!("{}/in_{}.{}", tmp_dir, uuid::Uuid::new_v4(), input_ext);
            let out_path = format!("{}/out_{}.wav", tmp_dir, uuid::Uuid::new_v4());
            std::fs::write(&in_path, &input_bytes)?;
            let status = std::process::Command::new(&ffmpeg)
                .args([
                    "-y", "-i", &in_path, "-ar", "16000", "-ac", "1", "-f", "wav", &out_path,
                ])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map_err(|_| AppError::Internal("ffmpeg not available".into()))?;
            let _ = std::fs::remove_file(&in_path);
            if !status.success() {
                let _ = std::fs::remove_file(&out_path);
                return Err(AppError::BadRequest("音频转码失败".into()));
            }
            let bytes = std::fs::read(&out_path)?;
            let _ = std::fs::remove_file(&out_path);
            Ok(bytes)
        })
        .await
        .map_err(|e| AppError::Internal(format!("转码任务失败: {}", e)))?
    }

    fn to_opus(&self, wav: &[u8], wav_path: &str, opus_path: &str) -> Option<Vec<u8>> {
        std::fs::write(wav_path, wav).ok()?;
        let status = std::process::Command::new(&self.ffmpeg_bin)
            .args([
                "-y", "-i", wav_path, "-c:a", "libopus", "-b:a", "64k", opus_path,
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .ok()?;
        let _ = std::fs::remove_file(wav_path);
        if status.success() {
            std::fs::read(opus_path).ok()
        } else {
            let _ = std::fs::remove_file(opus_path);
            None
        }
    }

    // ---------- ASR ----------

    /// 上传 16k wav → 识别文本。服务不可用 → Err(AppError::AsrUnavailable)
    pub async fn asr_recognize(&self, wav_16k: Vec<u8>) -> AppResult<AsrOutcome> {
        let url = format!("{}/recognize", self.asr_url.trim_end_matches('/'));
        let timeout = self.timeout;
        tokio::task::spawn_blocking(move || {
            let resp = ureq::agent()
                .post(&url)
                .set("Content-Type", "audio/wav")
                .timeout(timeout)
                .send_bytes(&wav_16k)
                .map_err(classify_asr_err)?;
            if resp.status() != 200 {
                return Err(AppError::AsrUnavailable);
            }
            let body: serde_json::Value = resp.into_json().map_err(|_| AppError::AsrUnavailable)?;
            Ok(AsrOutcome {
                text: body
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                confidence: body.get("confidence").and_then(|v| v.as_f64()),
            })
        })
        .await
        .map_err(|e| AppError::Internal(format!("asr task: {}", e)))?
    }
}

pub struct AsrOutcome {
    pub text: String,
    pub confidence: Option<f64>,
}

fn classify_tts_err(e: ureq::Error) -> AppError {
    match e {
        ureq::Error::Status(503, _) | ureq::Error::Status(500, _) => AppError::TtsUnavailable,
        _ => AppError::Inference(format!("tts error: {}", e)),
    }
}

fn classify_asr_err(e: ureq::Error) -> AppError {
    match e {
        ureq::Error::Status(503, _) | ureq::Error::Status(500, _) => AppError::AsrUnavailable,
        _ => AppError::Inference(format!("asr error: {}", e)),
    }
}

/// 缓存 key hash（非密码学用途，仅缓存文件名）
fn cache_hash(s: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", h)
}
