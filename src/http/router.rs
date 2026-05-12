use axum::{
    Router,
    routing::{get, post},
};

use crate::startup::AppState;

use super::handlers;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/api/v1/data/shorten", post(handlers::shorten))
        .route("/{short_code}", get(handlers::redirect))
        .with_state(state)
}
