//! BabyEng 业务后端入口（PRD 9.1 / 9.3）
//! Rust(axum) + SQLite(sqlx) + 推理服务分离（HTTP 调用 TTS/ASR/LLM）

mod config;
mod db;
mod error;
mod inference;
mod logic;
mod matcher;
mod models;
mod normalize;
mod routes;
mod state;
mod store;

use std::time::Duration;

use axum::Router;
use tower_http::trace::TraceLayer;
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,sqlx=warn".into()),
        )
        .init();

    let cfg = config::Config::from_env();
    let state = match state::AppState::new(cfg.clone()).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("启动失败: {}", e);
            std::process::exit(1);
        }
    };

    // 后台任务：定期刷新推理服务就绪状态（PRD 5.4「正在启动」提示条依据）
    {
        let state = state.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_secs(10));
            loop {
                tick.tick().await;
                state.inference.refresh_ready();
            }
        });
    }

    // api_router 内部已 with_state，返回 Router<()>；
    // 静态资源用 fallback_service（axum 0.8 根路径嵌套改为 fallback），
    // 未命中静态文件时回退 index.html（SPA 路由，PWA 可直接刷新子路径）。
    // 注意：ServeDir 的 not_found_service 会强制 404 状态码，SPA 回退须用 .fallback()
    let index_file = format!("{}/index.html", cfg.static_dir);
    let fallback = tower_http::services::ServeDir::new(&cfg.static_dir)
        .fallback(tower_http::services::ServeFile::new(&index_file));
    let app = Router::new()
        .merge(routes::api_router(state.clone()))
        .layer(TraceLayer::new_for_http())
        .fallback_service(fallback);

    let addr = &cfg.bind_addr;
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap_or_else(|e| {
            tracing::error!("监听 {} 失败: {}", addr, e);
            std::process::exit(1);
        });
    info!("BabyEng backend listening on http://{}", addr);
    info!("static dir: {}", cfg.static_dir);
    axum::serve(listener, app).await.unwrap();
}
