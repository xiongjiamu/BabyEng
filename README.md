# BabyEng · 幼儿英语启蒙教学应用

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
MVP 与 full 两个 profile，模型文件与数据卷分离。

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

- A2 别名命中 / A3 同音容错 / A4 未命中兜底：`backend/src/matcher.rs` 内单测可验证
- A5 录音闭环（静音自动停、双轨回放、母亲标记、<0.5s 不入库）：`views/Compare.vue` + `useRecorder.js`
- A7 降级不崩：手动停 TTS/ASR 服务后，前端分别给出降级提示
- A6/A8 需真机与人工试听：见 `deploy/README.md` 与 ROADMAP 待办
