//! 路由汇总

pub mod admin;
pub mod ask;
pub mod auth;
pub mod data;
pub mod family;
pub mod health;
pub mod progress;
pub mod recordings;
pub mod report;
pub mod tts;
pub mod unmatched;
pub mod words;

use axum::{middleware, Router};

use crate::state::SharedState;

pub fn api_router(state: SharedState) -> Router<()> {
    let protected = Router::new()
        .merge(auth::protected_router())
        .merge(admin::router())
        .merge(data::router())
        .merge(ask::router())
        .merge(tts::router())
        .merge(words::router())
        .merge(recordings::router())
        .merge(progress::router())
        .merge(report::router())
        .merge(family::router())
        .merge(unmatched::router())
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            crate::auth::require_auth,
        ));

    Router::new()
        .merge(health::router())
        .merge(auth::public_router())
        .merge(protected)
        .with_state(state)
}
