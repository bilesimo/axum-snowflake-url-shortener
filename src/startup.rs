use std::sync::Arc;

use axum::Router;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tokio::net::TcpListener;
use tracing::info;

use crate::{
    configuration::{DatabaseSettings, RedisSettings, Settings},
    domain::service::UrlShortenerService,
    error::AppError,
    http,
    id::generator::SequenceIdGenerator,
    storage::{postgres::PostgresUrlRepository, redis::RedisUrlCache},
};

#[derive(Clone)]
pub struct AppState {
    pub settings: Arc<Settings>,
    pub service: UrlShortenerService,
}

pub struct Application {
    port: u16,
    listener: TcpListener,
    router: Router,
}

impl Application {
    pub async fn build(mut settings: Settings) -> Result<Self, AppError> {
        let listener = TcpListener::bind(settings.application.bind_address())
            .await
            .map_err(|error| AppError::Internal(format!("failed to bind TCP listener: {error}")))?;
        let port = listener
            .local_addr()
            .map_err(|error| AppError::Internal(format!("failed to read local address: {error}")))?
            .port();

        if settings.application.base_url.trim().is_empty() {
            settings.application.base_url = format!("http://{}:{port}", settings.application.host);
        }

        let pool = get_connection_pool(&settings.database).await?;
        let repository = PostgresUrlRepository::new(pool.clone());
        repository.run_migrations().await?;

        let cache = RedisUrlCache::from_settings(&settings.redis)?;
        let id_generator = SequenceIdGenerator::new(pool, "short_urls_id_seq");
        let service = UrlShortenerService::new(repository, cache, id_generator);

        let router = http::router::build_router(AppState {
            settings: Arc::new(settings),
            service,
        });

        Ok(Self {
            port,
            listener,
            router,
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        info!("listening on {}", self.listener.local_addr()?);
        axum::serve(self.listener, self.router).await
    }
}

pub async fn get_connection_pool(settings: &DatabaseSettings) -> Result<PgPool, AppError> {
    PgPoolOptions::new()
        .max_connections(settings.max_connections)
        .connect_with(settings.with_db())
        .await
        .map_err(|error| AppError::Internal(format!("failed to connect to Postgres: {error}")))
}

pub fn build_cache(settings: &RedisSettings) -> Result<RedisUrlCache, AppError> {
    RedisUrlCache::from_settings(settings)
}
