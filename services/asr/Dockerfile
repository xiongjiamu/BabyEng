# BabyEng ASR 推理服务（sherpa-onnx，PRD 9.5）
FROM python:3.11-slim
WORKDIR /app
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt
COPY main.py .
ENV MODEL_DIR=/models/asr
EXPOSE 8102
CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8102"]
