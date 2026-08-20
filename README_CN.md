# BabyEng · 幼儿英语启蒙教学应用

[English](README.md) | 简体中文

面向 1~3 岁幼儿及其母亲的双角色英语启蒙工具。母亲用中文说一句「这是什么」「杯子英语怎么说」，
应用即时返回中文、英文、音标和英语发音，母亲跟着学一遍再教孩子，孩子跟读并录音回放对比。

对应文档：《幼儿英语启蒙教学应用 产品规范设计文档》v0.4（`docs/`），界面原型（`prototype/`）。

## 仓库结构

```
backend/    Rust 业务后端（axum + sqlx/SQLite + 匹配管线 L0~L2 + 掌握度/复习排期）
services/   Python 推理服务（tts=Piper、asr=sherpa-onnx、llm=OpenAI 兼容代理）
frontend/   Vue 3 + Vite PWA（12 页面，移植原型设计令牌）
data/       seed 内容数据（48 词 + 10 句 = 58 条，PRD 10.1 MVP 范围）
deploy/     docker-compose + nginx + Dockerfile + 部署文档
docs/       PRD 文档
prototype/  可交互 HTML 原型（设计来源）
```

## 本地开发

```bash
# 后端（需要 Rust 1.82+）
cd backend
cargo run   # 默认监听 127.0.0.1:8080，自动建库 + 导入 seed

# 前端（需要 Node 20+）
cd frontend
npm install
npm run dev # http://localhost:5173，/api 代理到 8080
```

后端启动参数（env）：

| 变量 | 默认 | 说明 |
|---|---|---|
| `BIND_ADDR` | 0.0.0.0:8080 | 监听地址 |
| `DATABASE_URL` | sqlite://data/babyeng.db | SQLite 路径 |
| `SEED_DIR` | data/seed | 词条 JSON 目录 |
| `AUDIO_DIR` | data/audio | 录音/音频缓存目录 |
| `TTS_URL` / `ASR_URL` / `LLM_URL` | 127.0.0.1:8101/2/3 | 推理服务地址 |
| `STATIC_DIR` | frontend/dist | PWA 产物（vite build 后） |

## 部署

见 `deploy/README.md`：docker compose 一键编排（web/backend/tts/asr/llm），
MVP 与 full 两个 profile，模型文件与数据卷分离。默认只监听宿主机
`127.0.0.1:80`；需要家庭局域网访问时必须显式设置 `WEB_BIND_ADDR` 为服务器的内网 IP。

本项目的 MVP 是单家庭应用，没有公网账号鉴权。不要直接暴露到公网；远程访问必须在前面增加 HTTPS 与独立访问控制层。

## 验证

```bash
cd backend
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo build --release

cd ../frontend
npm run build

cd ../deploy
npm install
npm run verify -- http://127.0.0.1:8080 /tmp/babyeng-shots
npm run release-check -- http://127.0.0.1:8080
```

自动检查不替代 PRD A6 的 58 条音频人工试听与音标核对，也不替代 A8 的安卓 Chrome、iOS Safari 真机闭环。

## TTS 模型引用与许可

BabyEng 的本地 TTS 服务使用 Michael Hansen 开发的 [Piper](https://github.com/rhasspy/piper) 以及
[rhasspy/piper-voices](https://huggingface.co/rhasspy/piper-voices) 发布的 ONNX 音色模型。若在研究、文章、演示或衍生项目中使用本项目，请同时引用 BabyEng、Piper 和实际使用音色的模型卡：

```bibtex
@software{piper_tts,
  author = {Michael Hansen},
  title = {Piper: A Fast, Local Neural Text to Speech System},
  url = {https://github.com/rhasspy/piper},
  year = {2023}
}

@inproceedings{kim2021vits,
  author = {Jaehyeon Kim and Jungil Kong and Juhee Son},
  title = {Conditional Variational Autoencoder with Adversarial Learning for End-to-End Text-to-Speech},
  booktitle = {Proceedings of the 38th International Conference on Machine Learning},
  pages = {5530--5540},
  year = {2021},
  url = {https://proceedings.mlr.press/v139/kim21f.html}
}
```

第二条为 Piper 所采用的 VITS 神经语音合成架构论文。

当前可选的 `en_US` medium 音色为 [Mike](https://huggingface.co/rhasspy/piper-voices/tree/main/en/en_US/mike/medium)、[Amy](https://huggingface.co/rhasspy/piper-voices/tree/main/en/en_US/amy/medium)、[Ryan](https://huggingface.co/rhasspy/piper-voices/tree/main/en/en_US/ryan/medium)、[Kristin](https://huggingface.co/rhasspy/piper-voices/tree/main/en/en_US/kristin/medium)、[HFC Female](https://huggingface.co/rhasspy/piper-voices/tree/main/en/en_US/hfc_female/medium) 和 [HFC Male](https://huggingface.co/rhasspy/piper-voices/tree/main/en/en_US/hfc_male/medium)。模型文件不会随本仓库分发。每个音色的模型卡分别记录其训练数据来源与许可；其中 Ryan、HFC Female 和 HFC Male 的模型卡标注了非商业许可。部署或再分发模型前请核对对应模型卡，本项目的 MIT License 不覆盖第三方模型、数据集、依赖或生成音频。

## License

本项目原创代码与文档采用 [MIT License](LICENSE)。第三方依赖、模型、数据集及媒体素材适用各自的许可条款。

## 需求覆盖速查

| PRD 章节 | 落地位置 |
|---|---|
| 4.1 语音问答（M1） | `backend/src/routes/ask.rs` + `frontend/src/views/Ask.vue` |
| 4.1.1 匹配管线 L0~L2 | `backend/src/normalize.rs` + `backend/src/matcher.rs` |
| 4.1.2 音标来源约束 | `data/seed/*.json`（phonetic_source 均非 llm）|
| 4.1.3 性能预算/降级 | 前端 800ms 回显 + TTS/ASR 降级分支 |
| 4.2 学习模式（M2） | `views/WordLearn.vue` / `Review.vue` / `Sentences.vue` |
| 4.3 录音对比（M3） | `views/Compare.vue` + `composables/useRecorder.js`（VAD 静音停）|
| 4.4 / 9.4 TTS | `services/tts/main.py`（Piper，语速可调）|
| 6.6 纯音频模式 | `views/AudioOnly.vue` |
| 6.7 首次引导 | `views/Onboarding.vue` |
| 7.1 成就/打卡/Streak Freeze | `backend/src/logic.rs` + `views/Profile.vue` |
| 7.3 学习日报（M8） | `backend/src/routes/report.rs` |
| 8.1~8.9 数据结构 | `backend/migrations/0001_init.sql` |
| 8.6 掌握度算法 | `backend/src/logic.rs::compute_mastery` |
| 8.8 未命中表 | `backend/src/routes/ask.rs::logic_unmatched` |
| 9.1~9.10 技术架构 | 本仓库全量（Rust + Python 推理分离 + Docker）|
| 11.3 屏幕时间 | 设置页上限 + 柔性收尾（`views/States.vue`）|
| 11.4 隐私/云端知情同意 | `views/Settings.vue` 二次确认弹窗 |

## 验收标准（PRD 10.1 A1~A8）

- A2 别名命中 / A3 同音容错 / A4 未命中兜底：`deploy/release-check.js`
- A5 录音闭环（静音自动停、双轨回放、母亲标记、<0.5s 不入库）：`views/Compare.vue` + `useRecorder.js`
- A7 降级不崩：手动停 TTS/ASR 服务后，前端分别给出降级提示
- A6/A8 需真机与人工试听：见 `deploy/README.md` 与 `ROADMAP.md` 待办
