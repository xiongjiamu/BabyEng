# BabyEng · English Learning for Toddlers

English | [简体中文](README_CN.md)

BabyEng is a local-first PWA for one family, designed for mothers and children aged 1–3. A parent can ask a question in Chinese, such as “What is this?” or “How do you say cup in English?”, and receive the Chinese term, English translation, phonetic transcription, and pronunciation. The parent learns first, models the word for the child, and can then record and compare the child's attempt.

The current target is the MVP release gate defined in Product Requirements Document v0.4 under `docs/`; V1 and V2 features are not release requirements.

## Repository layout

```text
backend/    Rust API (axum + sqlx/SQLite, L0–L2 matching, mastery and review scheduling)
services/   Python inference services (Piper TTS, sherpa-onnx ASR, OpenAI-compatible LLM proxy)
frontend/   Vue 3 + Vite PWA
data/       Reviewed seed content (48 words + 10 sentences for the MVP)
deploy/     Docker Compose, Nginx, Dockerfiles, deployment notes, and acceptance scripts
docs/       Product requirements and stable design documentation
prototype/  Interactive HTML prototype and original design reference
```

Only seed content with `review_status=published` is served by the application.

## Local development

```bash
# Backend (Rust 1.82+)
cd backend
cargo run

# Frontend (Node.js 20+)
cd frontend
npm install
npm run dev
```

The backend listens on `127.0.0.1:8080` during local development and initializes the SQLite database and seed content automatically. The Vite development server runs at `http://localhost:5173` and proxies `/api` to the backend.

Backend environment variables:

| Variable | Default | Purpose |
|---|---|---|
| `BIND_ADDR` | `0.0.0.0:8080` | Backend listen address |
| `DATABASE_URL` | `sqlite://data/babyeng.db` | SQLite database path |
| `SEED_DIR` | `data/seed` | Seed JSON directory |
| `AUDIO_DIR` | `data/audio` | Recordings and synthesized-audio cache |
| `TTS_URL` / `ASR_URL` / `LLM_URL` | ports `8101` / `8102` / `8103` | Inference service endpoints |
| `STATIC_DIR` | `frontend/dist` | Built PWA assets |

## Deployment and security boundary

See [`deploy/README.md`](deploy/README.md) for the Docker Compose deployment. The stack separates the web proxy, backend, TTS, ASR, and optional LLM services, and stores application data separately from downloaded model files.

BabyEng is intended for a trusted household network. The backend port is not exposed to the host by default and is reached through Nginx on the Compose network. Do not expose the application directly to the public Internet. Remote access requires HTTPS plus a separate access-control layer, such as a household VPN or authenticated reverse proxy. Recordings, child profile data, learning history, and unmatched questions are sensitive data concerning a minor.

Inference services are optional at runtime: if TTS or ASR is unavailable or its model is missing, the UI falls back to the text-based learning flow instead of failing the page.

## Verification

Run the release gate before merging code:

```bash
cd backend
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo build --release

cd ../frontend
npm run build

cd ..
python3 -m compileall -q services
docker compose -f deploy/docker-compose.yml config --quiet
```

When a local system Chrome installation is available, also run the page, API, and interaction smoke tests described in [`deploy/README.md`](deploy/README.md). Automated checks do not replace the PRD A6 manual review of all 58 pronunciations and phonetic transcriptions, or the A8 end-to-end checks on Android Chrome and iOS Safari. Do not report those gates as complete without retained manual evidence.

## TTS model attribution and citation

BabyEng's local TTS service uses [Piper](https://github.com/rhasspy/piper), created by Michael Hansen, with ONNX voice models distributed through [rhasspy/piper-voices](https://huggingface.co/rhasspy/piper-voices). If you use BabyEng in research, an article, a demonstration, or a derivative project, please cite BabyEng, Piper, and the model card for every voice you use. Piper can be cited as software:

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

The second entry cites the VITS neural speech-synthesis architecture used by Piper.

The selectable U.S. English medium-quality voices are:

| Voice | Model card | Dataset license stated by the model card |
|---|---|---|
| Mike (default) | [`en_US-mike-medium`](https://huggingface.co/rhasspy/piper-voices/tree/main/en/en_US/mike/medium) | CC0 |
| Amy | [`en_US-amy-medium`](https://huggingface.co/rhasspy/piper-voices/tree/main/en/en_US/amy/medium) | See the linked upstream dataset information |
| Ryan | [`en_US-ryan-medium`](https://huggingface.co/rhasspy/piper-voices/tree/main/en/en_US/ryan/medium) | CC BY-NC-SA 4.0 |
| Kristin | [`en_US-kristin-medium`](https://huggingface.co/rhasspy/piper-voices/tree/main/en/en_US/kristin/medium) | Public domain |
| HFC Female | [`en_US-hfc_female-medium`](https://huggingface.co/rhasspy/piper-voices/tree/main/en/en_US/hfc_female/medium) | CC BY-NC-SA 4.0 |
| HFC Male | [`en_US-hfc_male-medium`](https://huggingface.co/rhasspy/piper-voices/tree/main/en/en_US/hfc_male/medium) | CC BY-NC-SA 4.0 |

Model files are downloaded separately and are not distributed in this repository. Review the current model card and all upstream terms before deploying or redistributing a voice, especially for commercial use. BabyEng's MIT License does not grant rights to third-party models, training datasets, dependencies, or generated audio.

## License

BabyEng's original source code and documentation are available under the [MIT License](LICENSE). Third-party dependencies, models, datasets, and media remain subject to their respective terms.
