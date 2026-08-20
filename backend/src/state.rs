//! 应用共享状态

use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::RwLock;

use crate::config::Config;
use crate::inference::InferenceClients;
use crate::matcher::Matcher;

pub struct AppState {
    pub cfg: Config,
    pub pool: SqlitePool,
    pub matcher: RwLock<Matcher>,
    pub inference: InferenceClients,
}

pub type SharedState = Arc<AppState>;

impl AppState {
    pub async fn new(cfg: Config) -> crate::error::AppResult<SharedState> {
        let pool = crate::db::connect(&cfg).await?;
        crate::db::seed_if_empty(&pool, &cfg.seed_dir).await?;

        // 加载词句到匹配器
        let words = crate::store::load_words(&pool).await?;
        let sentences = crate::store::load_sentences(&pool).await?;
        let matcher = Matcher::new(&words, &sentences);

        let inference = InferenceClients::new(&cfg);
        inference.refresh_ready();

        Ok(Arc::new(AppState {
            cfg,
            pool,
            matcher: RwLock::new(matcher),
            inference,
        }))
    }
}
