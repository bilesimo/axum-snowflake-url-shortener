use axum::{
    Router,
    routing::{get, post},
};

use crate::AppState;

use super::handlers;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/api/v1/data/shorten", post(not_implemented_handler))
        .with_state(state)
}

async fn not_implemented_handler() -> &'static str {
    "not implemented"
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    use crate::{AppState, app_router, config::AppConfig};

    #[tokio::test]
    async fn health_endpoint_returns_ok() {
        let app = app_router(AppState {
            config: std::sync::Arc::new(AppConfig::from_env().expect("config")),
        });

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
    }
}
