mod config;
mod domain;
mod error;
mod http;
mod id;
mod storage;

use std::sync::Arc;

use axum::Router;
use config::AppConfig;
use domain::service::UrlShortenerService;
use id::generator::SequenceIdGenerator;
use storage::{UrlCache, UrlRepository, postgres::PostgresUrlRepository, redis::RedisUrlCache};
use tokio::net::TcpListener;
use tracing::info;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub service: Arc<UrlShortenerService>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = Arc::new(AppConfig::from_env()?);
    let postgres_repository = PostgresUrlRepository::connect(&config.database_url, 5).await?;
    postgres_repository.run_migrations().await?;

    let repository: Arc<dyn UrlRepository> = Arc::new(postgres_repository.clone());
    let cache: Arc<dyn UrlCache> = Arc::new(RedisUrlCache::from_url(&config.redis_url)?);
    let id_generator = Arc::new(SequenceIdGenerator::new(
        postgres_repository.pool().clone(),
        "short_urls_id_seq",
    ));
    let service = Arc::new(UrlShortenerService::new(repository, cache, id_generator));

    let app = app_router(AppState {
        config: Arc::clone(&config),
        service,
    });

    let listener = TcpListener::bind(config.bind_address()).await?;
    info!("listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await?;

    Ok(())
}

pub fn app_router(state: AppState) -> Router {
    http::router::build_router(state)
}
