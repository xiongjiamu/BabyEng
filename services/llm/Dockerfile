# BabyEng LLM 推理服务（OpenAI 兼容代理，可选，PRD 9.6）
FROM python:3.11-slim
WORKDIR /app
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt
COPY main.py .
EXPOSE 8103
CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8103"]
