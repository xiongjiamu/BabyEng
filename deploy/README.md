# BabyEng 部署文档（PRD 9.9 Docker 编排）

## 架构

```
[移动端 PWA] ──HTTPS── [Nginx(web)] ──/api── [backend Rust:8080]
                                    │
                        [SQLite /data/babyeng.db]
                        [TTS cache + OpenRouter Flux fallback]
                        [ASR sherpa :8102]
                        [LLM 可选   :8103, profile=full]
```

## 快速开始

```bash
# 1. 准备模型文件（体积较大，见下方「模型下载」）
mkdir -p models/piper models/asr

# 2. 启动 MVP（无 LLM，个人服务器最低配置可跑）
cd deploy
cp .env.example .env
cp auth.example.json auth.json
# 编辑 auth.json，为每个家庭配置不同的账号和高强度密码
docker compose --profile mvp up -d --build

# 3. 浏览器访问
#    默认仅服务器本机：http://127.0.0.1
#    家庭局域网：先设置 WEB_BIND_ADDR=<服务器内网IP>，再启动 Compose
#    录音需要 HTTPS（PRD 9.2），配置 Let's Encrypt 后改 https://babyeng.home.lan
```

## 访问边界

BabyEng 使用 `auth.json` 中的本地账号登录。Compose 默认只把 Web 端口绑定到 `127.0.0.1`，后端 8080 不暴露给宿主机。家庭局域网使用时，将 `WEB_BIND_ADDR` 显式设为服务器内网 IP；不要设为 `0.0.0.0` 后直接映射公网。

`.env.example` 默认监听全部宿主机网卡的 `18080` 端口，适合由服务器防火墙或云安全组限制来源网段的内网部署。若没有外层网络访问限制，应把 `WEB_BIND_ADDR` 改回服务器内网 IP 或 `127.0.0.1`。

`auth.json` 支持多组账号，格式见 `auth.example.json`。文件在每次登录时重新读取，修改账号密码无需重启；已有会话会持续到退出或后端重启。每个账号首次登录后拥有独立家庭、孩子、学习记录、录音、设置、导出和清理范围。账号名用于稳定关联数据，修改账号名会创建新的空数据空间，因此只应修改密码。该文件含明文密码，权限建议设为 `chmod 600 auth.json`，且已被 Git 忽略。

远程访问必须同时具备 HTTPS 和独立访问控制层，例如家庭 VPN 或反向代理身份验证。仅配置 TLS 不能替代访问控制。录音、孩子资料和学习记录都属于未成年人敏感数据。

## 模型下载

Piper 模型只用于复用已有的本地发音缓存（每个音色约 60MB）。新内容不再调用 Piper 实时生成：
```bash
# 下载应用可选的 6 个美式英语音色；Mike 为默认音色
mkdir -p models/piper
for voice in mike amy ryan kristin hfc_female hfc_male; do
  wget "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/${voice}/medium/en_US-${voice}-medium.onnx" -O "models/piper/en_US-${voice}-medium.onnx"
  wget "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/${voice}/medium/en_US-${voice}-medium.onnx.json" -O "models/piper/en_US-${voice}-medium.onnx.json"
done
```

ASR（sherpa-onnx 流式中文，约 300MB）：
```bash
# k2 双语中文-英文流式模型（PRD 9.5 首选）
cd models/asr
wget https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-streaming-zipformer-bilingual-zh-en-2023-02-20.tar.bz2
tar xjf sherpa-onnx-streaming-zipformer-bilingual-zh-en-2023-02-20.tar.bz2
rm sherpa-onnx-streaming-zipformer-bilingual-zh-en-2023-02-20.tar.bz2
```

**模型缺失时应用仍可用**：配置 `OPENROUTER_API_KEY` 后，未收录发音会由 OpenRouter Flux TTS 生成并缓存；OpenRouter 或 ASR 不可用时，前端自动降级——
问一问改打字、发音显示「暂时不可用」，不影响文字教学闭环（PRD 4.1.3 / 5.4）。

## 环境变量

| 变量 | 默认 | 说明 |
|---|---|---|
| `WEB_BIND_ADDR` | 127.0.0.1 | Web 监听地址；家庭局域网使用时填写服务器内网 IP |
| `WEB_PORT` | 80 | Web 宿主机端口；端口冲突时可改为 8080 等空闲端口 |
| `AUTH_FILE` | ./auth.json | 宿主机账号配置文件路径，只读挂载到后端 |
| `OPENROUTER_API_KEY` | 空 | OpenRouter Key；本地音频未收录时固定用 `deepgram/flux-tts:free` 的 `flux-drew-en` 生成 |
| `LLM_BASE_URL` | http://host.docker.internal:11434/v1 | 本地 ollama 或云端 OpenAI 兼容地址（full profile） |
| `LLM_API_KEY` | 空 | 云端 API Key（full profile） |
| `LLM_MODEL` | qwen2.5:7b-instruct | 模型名（full profile） |

## 数据与备份

- SQLite：`data` volume（`/data/babyeng.db`）
- 录音与 TTS 缓存：`audio` volume（`/data/audio`）
- 模型文件：`models` volume（首次需手动放入，不随镜像分发）

设置页“导出全部数据”会生成 `babyeng-backup.json`，其中包含家庭资料、学习记录、进度、未命中记录以及 Base64 编码的录音。服务器级灾备还应拷贝以上三个 volume 与环境变量。定时脚本建议：

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

HTTPS 配置不会自动增加账号鉴权；远程访问仍需使用家庭 VPN 或独立访问控制层。

## Release 验证

```bash
npm install
AUTH_USERNAME=family-a AUTH_PASSWORD='你的密码' npm run verify -- http://127.0.0.1:8080 /tmp/babyeng-shots
AUTH_USERNAME=family-a AUTH_PASSWORD='你的密码' npm run release-check -- http://127.0.0.1:8080
```

`verify` 检查 12 个页面、主要交互和 API；`release-check` 自动覆盖 A2 全量别名、A3 十例同音容错与 A4 十例未命中。它输出的 A1 数值只是本机文字 API 基线，不能替代局域网中端安卓语音链路的 P95 验收。

A6 必须人工逐条试听 58 条 TTS 音频并核对音标；A8 必须在安卓 Chrome 与 iOS Safari 各完成完整闭环并记录 PRD 9.2 的四项结论。

## 无 GPU 说明（PRD 9.7）

无 GPU 时不建议跑本地 LLM（CPU 上 7B 量化每秒个位数 token，与实时问答量级不匹配）。
MVP 默认路径是「词库 + 别名表做厚」，未命中走相近词推荐 + 文字输入 + 未命中表（8.8）。
云端 LLM 是可选项：设置页开启时会有一次知情同意确认（11.4），仅外发归一后的提问文本。
