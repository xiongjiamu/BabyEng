# BabyEng ASR 推理服务（PRD 9.5：首选 sherpa-onnx 流式中文模型）
# 模型文件挂载到 /models/asr/，缺失时返回 503 由后端降级为「没听清，可以打字」（5.4）
# 依赖：pip install sherpa-onnx fastapi uvicorn
# 契约：POST /recognize，Content-Type: audio/wav，body 为 16k 单声道 wav（后端已用 ffmpeg 转码）

import os
from pathlib import Path

import numpy as np
from fastapi import FastAPI, HTTPException, Request
from fastapi.responses import JSONResponse

app = FastAPI(title="BabyEng ASR", version="0.4.0")

MODEL_DIR = Path(os.environ.get("MODEL_DIR", "/models/asr"))

# sherpa-onnx 流式中文模型（zipformer bilingual zh-en 等）
MODEL_NAME = os.environ.get("ASR_MODEL", "sherpa-onnx-streaming-zipformer-bilingual-zh-en-2023-02-20")

_recognizer = None
_ready = False


def _load():
    global _recognizer, _ready
    try:
        import sherpa_onnx

        tokens = MODEL_DIR / MODEL_NAME / "tokens.txt"
        encoder = MODEL_DIR / MODEL_NAME / "encoder-epoch-99-avg-1.onnx"
        decoder = MODEL_DIR / MODEL_NAME / "decoder-epoch-99-avg-1.onnx"
        joiner = MODEL_DIR / MODEL_NAME / "joiner-epoch-99-avg-1.onnx"
        if not all(p.exists() for p in (tokens, encoder, decoder, joiner)):
            _ready = False
            return

        recognizer = sherpa_onnx.OnlineRecognizer.from_transducer(
            tokens=str(tokens),
            encoder=str(encoder),
            decoder=str(decoder),
            joiner=str(joiner),
            num_threads=2,
            sample_rate=16000,
            feature_dim=80,
            enable_endpoint_detection=True,
            rule1_min_trailing_silence=2.4,
            rule2_min_trailing_silence=1.2,
            rule3_min_utterance_length=300,
        )
        _recognizer = recognizer
        _ready = True
    except Exception as e:  # noqa: BLE001
        print(f"[asr] load failed: {e}")
        _recognizer = None
        _ready = False


_load()


@app.get("/healthz")
def healthz():
    return {"ok": True}


@app.get("/readyz")
def readyz():
    return {"ok": _ready}


@app.post("/recognize")
async def recognize(request: Request):
    """16k 单声道 wav → 中文文本 + 置信度。模型缺失 → 503"""
    if not _ready:
        raise HTTPException(status_code=503, detail="asr model not ready")
    body = await request.body()
    if len(body) < 44:
        raise HTTPException(status_code=400, detail="wav too short")

    # 解析 wav 头
    sample_rate = int.from_bytes(body[24:28], "little")
    channels = int.from_bytes(body[22:24], "little")
    if sample_rate != 16000:
        # 宽松：若采样率不符，重采样代价高，先按 16k 读（后端已保证 16k）
        pass

    samples = np.frombuffer(body[44:], dtype=np.int16).astype(np.float32) / 32768.0
    stream = _recognizer.create_stream()
    # 分块喂入（模拟流式）
    chunk = 1600  # 0.1s
    for i in range(0, len(samples), chunk):
        stream.accept_waveform(16000, samples[i : i + chunk])
        while _recognizer.is_ready(stream):
            _recognizer.decode_stream(stream)
    # 追加尾部静音并结束输入，让最后一个词有足够右上下文完成解码。
    stream.accept_waveform(16000, np.zeros(6400, dtype=np.float32))
    stream.input_finished()
    while _recognizer.is_ready(stream):
        _recognizer.decode_stream(stream)

    result = _recognizer.get_result(stream)
    text = (result.text if hasattr(result, "text") else str(result)).strip()
    # 置信度：本实现给出启发式（非空文本 0.8，空文本 0.0）
    confidence = 0.8 if text else 0.0
    return JSONResponse({"text": text, "confidence": confidence})


if __name__ == "__main__":
    import uvicorn

    uvicorn.run(app, host="0.0.0.0", port=int(os.environ.get("PORT", "8102")))
