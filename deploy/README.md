# BabyEng 部署文档（PRD 9.9 Docker 编排）

## 架构

```
[移动端 PWA] ──HTTPS── [Nginx(web)] ──/api── [backend Rust:8080]
                                    │
                        [SQLite /data/babyeng.db]
                        [TTS Piper  :8101]
                        [ASR sherpa :8102]
                        [LLM 可选   :8103, profile=full]
```

## 快速开始

```bash
# 1. 准备模型文件（体积较大，见下方「模型下载」）
mkdir -p models/piper models/asr

# 2. 启动 MVP（无 LLM，个人服务器最低配置可跑）
cd deploy
docker compose --profile mvp up -d --build

# 3. 浏览器访问
#    手机与服务器同局域网：http://<服务器IP>
#    录音需要 HTTPS（PRD 9.2），配置 Let's Encrypt 后改 https://babyeng.home.lan
```

## 模型下载

TTS（Piper，约 100~200MB）：
```bash
# en_US-lessig-medium：发音清晰、语速可调，PRD 4.4 首选
wget https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessig/medium/en_US-lessig-medium.onnx -O models/piper/en_US-lessig-medium.onnx
wget https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessig/medium/en_US-lessig-medium.onnx.json -O models/piper/en_US-lessig-medium.onnx.json
```

ASR（sherpa-onnx 流式中文，约 300MB）：
```bash
# k2 双语中文-英文流式模型（PRD 9.5 首选）
# 从 k2-fsa/sherpa-onnx 仓库下载 sherpa-onnx-streaming-zipformer-bilingual-zh-en-2023-02-20 解压到 models/asr/
```

**模型缺失时应用仍可用**：TTS/ASR 返回 503，前端自动降级——
问一问改打字、发音显示「暂时不可用」，不影响文字教学闭环（PRD 4.1.3 / 5.4）。

## 环境变量

| 变量 | 默认 | 说明 |
|---|---|---|
| `MASTER_KEY` | change-me-in-prod | API Key 加密主密钥（9.10） |
| `LLM_BASE_URL` | http://host.docker.internal:11434/v1 | 本地 ollama 或云端 OpenAI 兼容地址（full profile） |
| `LLM_API_KEY` | 空 | 云端 API Key（full profile） |
| `LLM_MODEL` | qwen2.5:7b-instruct | 模型名（full profile） |

## 数据与备份

- SQLite：`data` volume（`/data/babyeng.db`）
- 录音与 TTS 缓存：`audio` volume（`/data/audio`）
- 模型文件：`models` volume（首次需手动放入，不随镜像分发）

备份 = 拷贝以上三个 volume + 环境变量。定时脚本建议：

```bash
docker run --rm -v babyeng_data:/data -v babyeng_audio:/audio -v $PWD/backup:/backup alpine \
  sh -c "cp /data/babyeng.db /backup/ && tar czf /backup/audio.tar.gz -C /audio ."
```

## HTTPS（录音必需）

```bash
# 方案 A：Let's Encrypt（推荐，有公网域名）
sudo certbot --nginx -d babyeng.home.lan

# 方案 B：内网自签（无公网域名）
# 生成自签证书并在手机本地信任；nginx 增加 443 server 块
```

## 无 GPU 说明（PRD 9.7）

无 GPU 时不建议跑本地 LLM（CPU 上 7B 量化每秒个位数 token，与实时问答量级不匹配）。
MVP 默认路径是「词库 + 别名表做厚」，未命中走相近词推荐 + 文字输入 + 未命中表（8.8）。
云端 LLM 是可选项：设置页开启时会有一次知情同意确认（11.4），仅外发归一后的提问文本。
