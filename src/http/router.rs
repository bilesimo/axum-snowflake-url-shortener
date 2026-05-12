use axum::{
    Router,
    routing::{get, post},
};

use crate::AppState;

use super::handlers;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/api/v1/data/shorten", post(handlers::shorten))
        .route("/{short_code}", get(handlers::redirect))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use axum::{
        body::Body,
        http::{Request, StatusCode, header::LOCATION},
    };
    use http_body_util::BodyExt;
    use serde_json::json;
    use tokio::sync::Mutex;
    use tower::ServiceExt;

    use crate::{
        AppState, app_router,
        config::AppConfig,
        domain::{
            model::{NewShortUrl, ShortUrl},
            service::UrlShortenerService,
        },
        error::AppError,
        id::generator::{IdGenerator, InMemoryIdGenerator},
        storage::{UrlCache, UrlRepository},
    };
    use async_trait::async_trait;

    #[derive(Default)]
    struct FakeRepository {
        by_short_code: Mutex<HashMap<String, ShortUrl>>,
        by_normalized_long_url: Mutex<HashMap<String, ShortUrl>>,
    }

    #[async_trait]
    impl UrlRepository for FakeRepository {
        async fn find_by_short_code(&self, short_code: &str) -> Result<Option<ShortUrl>, AppError> {
            Ok(self.by_short_code.lock().await.get(short_code).cloned())
        }

        async fn find_by_normalized_long_url(
            &self,
            normalized_long_url: &str,
        ) -> Result<Option<ShortUrl>, AppError> {
            Ok(self
                .by_normalized_long_url
                .lock()
                .await
                .get(normalized_long_url)
                .cloned())
        }

        async fn insert(&self, new_short_url: &NewShortUrl) -> Result<ShortUrl, AppError> {
            let short_url = ShortUrl {
                id: new_short_url.id,
                short_code: new_short_url.short_code.clone(),
                long_url: new_short_url.long_url.clone(),
                normalized_long_url: new_short_url.normalized_long_url.clone(),
            };

            self.by_short_code
                .lock()
                .await
                .insert(short_url.short_code.clone(), short_url.clone());
            self.by_normalized_long_url
                .lock()
                .await
                .insert(short_url.normalized_long_url.clone(), short_url.clone());

            Ok(short_url)
        }
    }

    #[derive(Default)]
    struct FakeCache {
        entries: Mutex<HashMap<String, String>>,
    }

    #[async_trait]
    impl UrlCache for FakeCache {
        async fn get_long_url(&self, short_code: &str) -> Result<Option<String>, AppError> {
            Ok(self.entries.lock().await.get(short_code).cloned())
        }

        async fn set_long_url(&self, short_code: &str, long_url: &str) -> Result<(), AppError> {
            self.entries
                .lock()
                .await
                .insert(short_code.to_owned(), long_url.to_owned());
            Ok(())
        }
    }

    fn test_state() -> AppState {
        let config = Arc::new(AppConfig::from_env().expect("config"));
        let repository: Arc<dyn UrlRepository> = Arc::new(FakeRepository::default());
        let cache: Arc<dyn UrlCache> = Arc::new(FakeCache::default());
        let id_generator: Arc<dyn IdGenerator> = Arc::new(InMemoryIdGenerator::new(1));
        let service = Arc::new(UrlShortenerService::new(repository, cache, id_generator));

        AppState { config, service }
    }

    #[tokio::test]
    async fn health_endpoint_returns_ok() {
        let app = app_router(test_state());

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

    #[tokio::test]
    async fn shorten_endpoint_returns_created_mapping() {
        let app = app_router(test_state());

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/data/shorten")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({ "long_url": "https://example.com/articles/123" }).to_string(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes();
        let payload: serde_json::Value = serde_json::from_slice(&body).expect("json body");

        assert_eq!(payload["short_code"], "1");
        assert_eq!(payload["short_url"], "http://127.0.0.1:3000/1");
        assert_eq!(payload["long_url"], "https://example.com/articles/123");
    }

    #[tokio::test]
    async fn redirect_endpoint_returns_location_header() {
        let app = app_router(test_state());

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/data/shorten")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({ "long_url": "https://example.com/redirect-me" }).to_string(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("create response");

        assert_eq!(create_response.status(), StatusCode::CREATED);

        let redirect_response = app
            .oneshot(
                Request::builder()
                    .uri("/1")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("redirect response");

        assert_eq!(redirect_response.status(), StatusCode::FOUND);
        assert_eq!(
            redirect_response.headers().get(LOCATION).and_then(|value| value.to_str().ok()),
            Some("https://example.com/redirect-me")
        );
    }
}
