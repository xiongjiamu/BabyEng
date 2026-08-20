//! 路由汇总

pub mod ask;
pub mod data;
pub mod family;
pub mod health;
pub mod progress;
pub mod recordings;
pub mod report;
pub mod tts;
pub mod unmatched;
pub mod words;

use axum::Router;

use crate::state::SharedState;

pub fn api_router(state: SharedState) -> Router<()> {
    Router::new()
        .merge(health::router())
        .merge(data::router())
        .merge(ask::router())
        .merge(tts::router())
        .merge(words::router())
        .merge(recordings::router())
        .merge(progress::router())
        .merge(report::router())
        .merge(family::router())
        .merge(unmatched::router())
        .with_state(state)
}
