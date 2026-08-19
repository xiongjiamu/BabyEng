# BabyEng LLM 推理服务（PRD 9.6 / 9.8：OpenAI 兼容代理，可选）
# 家庭配置云端 API（openai 兼容）或本地 ollama/llama.cpp，本服务统一转发并强制结构化输出。
# 无 GPU 默认不启用（9.7）；LLM 输出的音标字段一律丢弃（4.1.2）。
# 依赖：pip install fastapi uvicorn httpx

import json
import os

import httpx
from fastapi import FastAPI, HTTPException, Request
from fastapi.responses import JSONResponse

app = FastAPI(title="BabyEng LLM Proxy", version="0.4.0")

# OpenAI 兼容端点（ollama: http://host.docker.internal:11434/v1；云端：https://dashscope.aliyuncs.com/compatible-mode/v1）
BASE_URL = os.environ.get("OPENAI_BASE_URL", "")
API_KEY = os.environ.get("OPENAI_API_KEY", "")
MODEL = os.environ.get("LLM_MODEL", "qwen2.5:7b-instruct")

_ready = bool(BASE_URL)
client = httpx.Client(timeout=60.0)

# 固化 prompt 模板（11.2：结构化 JSON + 白名单校验；音标字段在模板里明确要求不输出）
PROMPT_TEMPLATE = """你是家庭英语启蒙应用的词条生成器。用户给出一个中文词或短语（可能是口语说法），
请输出 JSON，字段如下：
{
  "zh": "中文主词",
  "en": "对应英文（词或短句）",
  "example_en": "一个例句（幼儿日常生活场景）",
  "example_zh": "例句中文",
  "mother_tip": "给母亲的一句发音要点，不出现语法术语，只给可直接照说的句子"
}
约束：
1. 不输出音标（phonetic），音标由发音词典负责。
2. en 必须是小写、儿童日常高频用法。
3. 只输出 JSON，不要任何解释文字。
中文输入：{zh}
"""


@app.get("/healthz")
def healthz():
    return {"ok": True}


@app.get("/readyz")
def readyz():
    return {"ok": _ready}


@app.post("/generate")
async def generate(request: Request):
    if not _ready:
        raise HTTPException(status_code=503, detail="llm not configured")
    body = await request.json()
    zh = body.get("zh", "")
    if not zh:
        raise HTTPException(status_code=400, detail="zh required")

    try:
        resp = client.post(
            f"{BASE_URL.rstrip('/')}/chat/completions",
            headers={"Authorization": f"Bearer {API_KEY}"} if API_KEY else {},
            json={
                "model": MODEL,
                "messages": [
                    {
                        "role": "system",
                        "content": PROMPT_TEMPLATE.format(zh=zh),
                    },
                    {"role": "user", "content": zh},
                ],
                "temperature": 0.3,
                "response_format": {"type": "json_object"},
            },
            timeout=60.0,
        )
        resp.raise_for_status()
        content = resp.json()["choices"][0]["message"]["content"]
        parsed = json.loads(content)
        # 白名单校验（11.2）
        if not isinstance(parsed, dict) or not parsed.get("en"):
            raise ValueError("bad llm output")
        # 强制丢弃音标字段
        parsed.pop("phonetic", None)
        parsed.pop("phonetic_source", None)
        return JSONResponse(parsed)
    except Exception as e:  # noqa: BLE001
        raise HTTPException(status_code=502, detail=f"llm upstream error: {e}")


if __name__ == "__main__":
    import uvicorn

    uvicorn.run(app, host="0.0.0.0", port=int(os.environ.get("PORT", "8103")))
