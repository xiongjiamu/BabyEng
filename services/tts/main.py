# BabyEng TTS 推理服务（PRD 9.4：首选 Piper ONNX）
# 模型文件挂载到 /models/piper/，缺失时返回 503 由后端降级为「发音暂时不可用」（4.1.3）
# 依赖：pip install piper-tts fastapi uvicorn

import io
import os
import wave
from pathlib import Path

from fastapi import FastAPI, HTTPException, Query
from fastapi.responses import Response

app = FastAPI(title="BabyEng TTS", version="0.4.0")

MODEL_DIR = Path(os.environ.get("MODEL_DIR", "/models/piper"))
DEFAULT_VOICE = os.environ.get("PIPER_VOICE", "en_US-lessac-medium")
ALLOWED_VOICES = {
    "en_US-lessac-medium",
    "en_US-amy-medium",
    "en_US-ryan-medium",
}

# 音色缓存：voice -> PiperVoice 实例
_voices = {}
_ready = False


def _load_voice(voice: str):
    """加载指定音色；成功返回实例，失败返回 None"""
    if voice not in ALLOWED_VOICES:
        return None
    try:
        from piper import PiperVoice
    except ImportError:
        return None
    model_path = MODEL_DIR / f"{voice}.onnx"
    config_path = MODEL_DIR / f"{voice}.onnx.json"
    if not (model_path.exists() and config_path.exists()):
        return None
    try:
        return PiperVoice.load(str(model_path), str(config_path))
    except Exception as e:  # noqa: BLE001
        print(f"[tts] load {voice} failed: {e}")
        return None


def _any_model_available() -> bool:
    if not MODEL_DIR.exists():
        return False
    if not DEFAULT_VOICE:
        return False
    return (MODEL_DIR / f"{DEFAULT_VOICE}.onnx").exists()


_default_voice = _load_voice(DEFAULT_VOICE)
if _default_voice is not None:
    _voices[DEFAULT_VOICE] = _default_voice
_ready = _default_voice is not None


@app.get("/healthz")
def healthz():
    return {"ok": True}


@app.get("/readyz")
def readyz():
    return {"ok": _ready, "voice": DEFAULT_VOICE}


@app.get("/synthesize")
def synthesize(
    text: str = Query(...),
    voice: str = Query(DEFAULT_VOICE),
    length_scale: float = Query(0.8, ge=0.5, le=1.5),
):
    """合成文本 → 16k 单声道 wav。模型缺失 → 503（后端降级提示）"""
    if not _ready:
        raise HTTPException(status_code=503, detail="tts model not ready")

    if voice not in _voices:
        v = _load_voice(voice)
        if v is None:
            raise HTTPException(status_code=503, detail=f"voice {voice} not available")
        _voices[voice] = v
    v = _voices[voice]

    wav_bytes = io.BytesIO()
    try:
        from piper.config import SynthesisConfig

        with wave.open(wav_bytes, "wb") as f:
            v.synthesize_wav(
                text,
                f,
                syn_config=SynthesisConfig(length_scale=length_scale),
            )
    except Exception as e:  # noqa: BLE001
        raise HTTPException(status_code=500, detail=f"synthesize error: {e}")
    return Response(
        content=wav_bytes.getvalue(),
        media_type="audio/wav",
        headers={"Cache-Control": "no-cache"},
    )


if __name__ == "__main__":
    import uvicorn

    uvicorn.run(app, host="0.0.0.0", port=int(os.environ.get("PORT", "8101")))
