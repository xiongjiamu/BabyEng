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
    pub openrouter_api_key: String,
    pub openrouter_tts_url: String,
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
            openrouter_api_key: cfg.openrouter_api_key.clone(),
            openrouter_tts_url: cfg.openrouter_tts_url.clone(),
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
            Ok(resp) if resp.status() == 200 => resp
                .into_json::<serde_json::Value>()
                .ok()
                .and_then(|body| body.get("ok").and_then(|ok| ok.as_bool()))
                .unwrap_or(false),
            Ok(_) | Err(_) => false,
        }
    }

    // ---------- TTS ----------

    pub fn tts_available(&self) -> bool {
        !self.openrouter_api_key.is_empty()
    }

    /// 通过 OpenRouter Flux TTS 合成未收录文本，固定使用 Drew 音色并返回 MP3。
    async fn flux_tts_synthesize(&self, text: String) -> AppResult<Vec<u8>> {
        if self.openrouter_api_key.is_empty() {
            return Err(AppError::TtsUnavailable);
        }
        let url = self.openrouter_tts_url.clone();
        let api_key = self.openrouter_api_key.clone();
        let timeout = self.timeout;
        tokio::task::spawn_blocking(move || {
            let authorization = format!("Bearer {}", api_key);
            let body = flux_tts_request(&text);
            let resp = ureq::agent()
                .post(&url)
                .set("Authorization", &authorization)
                .set("Content-Type", "application/json")
                .set("HTTP-Referer", "eng.xxm.mom")
                .set("X-Title", "BabyEng")
                .timeout(timeout)
                .send_json(body)
                .map_err(classify_flux_tts_err)?;
            if resp.status() != 200 {
                return Err(AppError::TtsUnavailable);
            }
            let mut buf = Vec::new();
            resp.into_reader()
                .read_to_end(&mut buf)
                .map_err(|_| AppError::TtsUnavailable)?;
            if buf.is_empty() {
                return Err(AppError::TtsUnavailable);
            }
            Ok(buf)
        })
        .await
        .map_err(|e| AppError::Internal(format!("flux tts task: {}", e)))?
    }

    /// 获取 TTS 音频：旧 Piper 缓存优先，未命中时用固定 Flux 模型和音色。
    /// 返回 (字节, 扩展名, 是否缓存命中)
    pub async fn tts_audio(
        &self,
        text: &str,
        voice: &str,
        rate: f64,
    ) -> AppResult<(Vec<u8>, String, bool)> {
        let cache_dir = format!("{}/tts_cache", self.audio_dir);
        std::fs::create_dir_all(&cache_dir).ok();

        // 优先复用旧 Piper 音色已收录的音频，再查固定 Flux 音色缓存。
        let legacy_key = format!("{}|{}|{:.2}", voice, text, rate);
        let flux_key = format!("deepgram/flux-tts:free|flux-drew-en|{}", text);
        for (hash, extensions) in [
            (cache_hash(&legacy_key), ["opus", "wav"]),
            (cache_hash(&flux_key), ["opus", "mp3"]),
        ] {
            for ext in extensions {
                let path = format!("{}/{}.{}", cache_dir, hash, ext);
                if Path::new(&path).exists() {
                    let bytes = std::fs::read(&path)?;
                    return Ok((bytes, ext.into(), true));
                }
            }
        }

        // 未收录 → OpenRouter Flux TTS 固定 Drew 音色 → 压 Opus，不使用用户 Piper 配置。
        let hash = cache_hash(&flux_key);
        let opus_path = format!("{}/{}.opus", cache_dir, hash);
        let mp3_path = format!("{}/{}.mp3", cache_dir, hash);
        let mp3 = self.flux_tts_synthesize(text.to_string()).await?;
        let converted = self.to_opus(&mp3, &mp3_path, &opus_path);
        match converted {
            Some(opus_bytes) => Ok((opus_bytes, "opus".into(), false)),
            None => {
                std::fs::write(&mp3_path, &mp3).ok();
                Ok((mp3, "mp3".into(), false))
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

fn classify_flux_tts_err(e: ureq::Error) -> AppError {
    match e {
        ureq::Error::Status(_, _) | ureq::Error::Transport(_) => AppError::TtsUnavailable,
    }
}

fn flux_tts_request(text: &str) -> serde_json::Value {
    serde_json::json!({
        "model": "deepgram/flux-tts:free",
        "input": text,
        "voice": "flux-drew-en",
        "response_format": "mp3"
    })
}

fn classify_asr_err(e: ureq::Error) -> AppError {
    match e {
        ureq::Error::Transport(_) => AppError::AsrUnavailable,
        ureq::Error::Status(status, _) if (500..=599).contains(&status) => AppError::AsrUnavailable,
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

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::time::Duration;

    use super::{flux_tts_request, InferenceClients, ReadyState};
    use crate::error::AppError;

    #[test]
    fn flux_request_uses_fixed_model_voice_and_mp3() {
        assert_eq!(
            flux_tts_request("A red cup."),
            serde_json::json!({
                "model": "deepgram/flux-tts:free",
                "input": "A red cup.",
                "voice": "flux-drew-en",
                "response_format": "mp3"
            })
        );
    }

    #[tokio::test]
    async fn cache_miss_calls_flux_once_and_ignores_later_piper_voice() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (request_tx, request_rx) = mpsc::channel();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut buf = [0_u8; 2048];
            loop {
                let count = stream.read(&mut buf).unwrap();
                if count == 0 {
                    break;
                }
                request.extend_from_slice(&buf[..count]);
                let Some(header_end) = request.windows(4).position(|w| w == b"\r\n\r\n") else {
                    continue;
                };
                let headers = String::from_utf8_lossy(&request[..header_end]);
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .and_then(|value| value.trim().parse::<usize>().ok())
                    })
                    .unwrap();
                if request.len() >= header_end + 4 + content_length {
                    break;
                }
            }
            request_tx
                .send(String::from_utf8(request).unwrap())
                .unwrap();
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Type: audio/mpeg\r\nContent-Length: 11\r\nConnection: close\r\n\r\nID3testdata")
                .unwrap();
        });

        let audio_dir = std::env::temp_dir()
            .join(format!("babyeng-flux-test-{}", uuid::Uuid::new_v4()))
            .to_string_lossy()
            .into_owned();
        let clients = InferenceClients {
            tts_url: String::new(),
            asr_url: String::new(),
            llm_url: String::new(),
            timeout: Duration::from_secs(2),
            ffmpeg_bin: "babyeng-test-ffmpeg-not-installed".into(),
            audio_dir,
            openrouter_api_key: "test-key".into(),
            openrouter_tts_url: format!("http://{}/api/v1/audio/speech", address),
            ready: std::sync::RwLock::new(ReadyState::default()),
        };

        let first = clients
            .tts_audio("A red cup.", "en_US-mike-medium", 0.8)
            .await
            .unwrap();
        let second = clients
            .tts_audio("A red cup.", "en_US-amy-medium", 1.2)
            .await
            .unwrap();

        assert_eq!(first, (b"ID3testdata".to_vec(), "mp3".into(), false));
        assert_eq!(second, (b"ID3testdata".to_vec(), "mp3".into(), true));
        let request = request_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let request_lower = request.to_ascii_lowercase();
        assert!(request.starts_with("POST /api/v1/audio/speech HTTP/1.1\r\n"));
        assert!(request_lower.contains("authorization: bearer test-key\r\n"));
        assert!(request_lower.contains("http-referer: eng.xxm.mom\r\n"));
        assert!(request_lower.contains("x-title: babyeng\r\n"));
        assert!(request.contains("\"model\":\"deepgram/flux-tts:free\""));
        assert!(request.contains("\"voice\":\"flux-drew-en\""));
    }

    #[tokio::test]
    async fn unreachable_asr_is_classified_as_unavailable_degradation() {
        let clients = InferenceClients {
            tts_url: String::new(),
            asr_url: "http://127.0.0.1:9".into(),
            llm_url: String::new(),
            timeout: Duration::from_secs(1),
            ffmpeg_bin: String::new(),
            audio_dir: std::env::temp_dir().to_string_lossy().into_owned(),
            openrouter_api_key: String::new(),
            openrouter_tts_url: String::new(),
            ready: std::sync::RwLock::new(ReadyState::default()),
        };
        let result = clients.asr_recognize(vec![0; 44]).await;
        assert!(matches!(result, Err(AppError::AsrUnavailable)));
    }
}
