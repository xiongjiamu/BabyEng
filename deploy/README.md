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
AUTH_USERNAME=family-a AUTH_PASSWORD='你的密码' npm run degradation-check -- http://127.0.0.1:18080
AUTH_USERNAME=family-a AUTH_PASSWORD='你的密码' npm run pronunciation-check -- http://127.0.0.1:18080 /tmp/babyeng-a6-pronunciation.json
npm run pronunciation-check -- --verify-manual /tmp/babyeng-a6-pronunciation.json
npm run device-evidence -- --generate /tmp/babyeng-device-evidence.json
npm run device-evidence -- --verify /tmp/babyeng-device-evidence.json
npm run custom-voice-readiness -- --generate /tmp/babyeng-custom-voice-readiness.json
npm run custom-voice-readiness -- --verify /tmp/babyeng-custom-voice-readiness.json
```

`verify` 检查 12 个页面、主要交互和 API；`release-check` 自动覆盖 A2 全量别名、A3 十例同音容错与 A4 十例未命中。它输出的 A1 数值只是本机文字 API 基线，不能替代局域网中端安卓语音链路的 P95 验收。

`degradation-check` 会依次停止 TTS、ASR 容器，检查文字路径始终可用、ASR 停止时语音接口明确返回 `asr_fail`，并在成功、失败或中断后恢复它停止的服务。执行前要求 Backend、TTS、ASR 均已运行；脚本不会启动原本就停止的环境。当前未收录发音使用 OpenRouter Flux，因此停止本地 TTS 后，配置了 Key 时应由远端继续提供发音，未配置时应返回 `tts_only_down` 并保留文字结果。自动输出中的 `manual_ui_evidence` 固定为 `false`，仍需人工记录移动端提示和恢复体验。

`pronunciation-check` 逐条请求 48 个单词和 10 个句子的实际 TTS，记录 HTTP 状态、音频 MIME、字节数和 SHA-256，并检查音标与来源字段。输出 JSON 权限为 `0600`，每条都预留人工试听、发音正确性、自然清晰度、音标核对、验收人和时间字段；这些字段初始为 `null`，脚本固定输出 `manual_complete=false`。人工填写后用 `--verify-manual` 检查是否恰好 58 条且所有结论、验收人和时间均已填写；自动通过 58 条只代表音频可读取，不能代替 A6 人工结论。

`device-evidence` 生成权限为 `0600` 的 A1/A5/A7/A8 与屏幕计时真机清单。A1 必须填写中端安卓 Chrome 的 20 次语音样本，校验器计算 P95 并执行 1200ms/800ms 阈值；A5、A7、A8 和屏幕计时必须同时有安卓与 iOS 的设备、浏览器、验收人、时间及逐项通过记录，iOS 四项 PWA 限制允许结论为 supported/limited/unavailable，但必须描述实测行为并确认应对路径。空模板不能通过 `--verify`，桌面模拟数据也不能替代真机字段。

`custom-voice-readiness` 只生成和校验自定义音色的人工决策清单，不读取、复制或上传录音，也不会启动训练。清单固定为本家庭成年母亲的本地英语 TTS，明确拒绝幼儿音色；必须填完说话人授权、20～30 分钟试采目标、原始/派生/模型/缓存/备份删除边界、隔离 GPU、训练器与 checkpoint 许可证以及批准人和时间。空白清单、允许幼儿声音、低于 8 GB VRAM、云训练或覆盖已有证据文件都会被拒绝。详细边界见 `docs/自定义音色准备与数据治理.md`；校验通过只代表决策完整，不代表已经采集、训练或获得可用音色。

A6 必须人工逐条试听 58 条 TTS 音频并核对音标；A7 仍需人工查看 TTS/ASR 故障时的界面降级；A8 必须在安卓 Chrome 与 iOS Safari 各完成完整闭环并记录 PRD 9.2 的四项结论。

管理后台可为已保存的单词和亲子活动上传 JPEG、PNG 或 WebP 实物照片，单张不超过 5 MB；替换和删除都需要明确确认。照片保存在 Backend 数据卷的 `/data/content-images`，应随数据库和录音一并备份。课程 JSON 导入导出不嵌入图片文件；迁移课程时需另行复制该目录。未上传照片时前端继续显示课程 emoji。
课程列表会汇总可配照片内容的覆盖率，并在每个单词/活动旁标记“有照片”或“待照片”；句子不支持照片并明确显示“无照片”。

管理后台的“使用证据”从数据库迁移 `0009_usage_evidence` 生效后开始按 7、28 或 90 天统计 PRD 13 指标。事件只保存输入方式、命中状态、课程目标、后端处理时长及问答—跟读关联，不额外复制提问原文；未命中原文仍只保存在原有待补词表中。未命中率以已经获得文本的提问为分母，`asr_fail` 只进入 ASR 成功率和总问答闭环率，避免把识别故障误判为词库缺口。闭环教学日要求同一问答事件关联到已保存的宝宝跟读录音。跟踪周从首条事件日期起每 7 天分组，无事件周显示为零，当前未满 7 天的周期标为“进行中”；只有首个 28 天完整结束后才显示前四周周均、第 4 周目标与止损结论。统计事件会进入家庭数据导出，也会随“清空学习数据”的明确确认一起删除。历史数据不会推测补齐，页面显示的后端 P95 不能替代 A1 真机端到端时延。
使用证据页同时提供“待补词清单”：管理员可按同一时间窗口查看 pending 未命中，结果按归一化文本跨家庭聚合并显示出现家庭数、次数和最近时间；接口不返回家庭原始提问或录音内容，普通家庭账号无权访问。该清单只读，新增课程仍需管理员在课程内容页明确导入/发布。

## 无 GPU 说明（PRD 9.7）

无 GPU 时不建议跑本地 LLM（CPU 上 7B 量化每秒个位数 token，与实时问答量级不匹配）。
MVP 默认路径是「词库 + 别名表做厚」，未命中走相近词推荐 + 文字输入 + 未命中表（8.8）。
云端 LLM 是可选项：设置页开启时会有一次知情同意确认（11.4），仅外发归一后的提问文本。
