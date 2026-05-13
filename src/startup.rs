use std::{net::SocketAddr, sync::Arc};

use axum::Router;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tokio::net::TcpListener;
use tracing::info;

use crate::{
    configuration::{DatabaseSettings, RedisSettings, Settings},
    domain::service::UrlShortenerService,
    error::AppError,
    http,
    id::generator::SnowflakeIdGenerator,
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
        let local_address = listener.local_addr().map_err(|error| {
            AppError::Internal(format!("failed to read local address: {error}"))
        })?;
        let port = local_address.port();

        if settings.application.base_url.trim().is_empty() {
            settings.application.base_url = derive_base_url(local_address)?;
        }

        let pool = get_connection_pool(&settings.database).await?;
        let repository = PostgresUrlRepository::new(pool.clone());
        repository.run_migrations().await?;

        let cache = RedisUrlCache::from_settings(&settings.redis).await?;
        let id_generator = SnowflakeIdGenerator::new(&settings.id)?;
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

fn derive_base_url(address: SocketAddr) -> Result<String, AppError> {
    if address.ip().is_unspecified() {
        return Err(AppError::Configuration(
            "application.base_url must be set when binding to an unspecified host".to_owned(),
        ));
    }

    Ok(format!("http://{address}"))
}

pub async fn get_connection_pool(settings: &DatabaseSettings) -> Result<PgPool, AppError> {
    PgPoolOptions::new()
        .max_connections(settings.max_connections)
        .connect_with(settings.with_db())
        .await
        .map_err(|error| AppError::Internal(format!("failed to connect to Postgres: {error}")))
}

pub async fn build_cache(settings: &RedisSettings) -> Result<RedisUrlCache, AppError> {
    RedisUrlCache::from_settings(settings).await
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv6Addr, SocketAddr};

    use crate::error::AppError;

    use super::derive_base_url;

    #[test]
    fn rejects_unspecified_ipv4_bind_when_base_url_is_missing() {
        let address = SocketAddr::from(([0, 0, 0, 0], 3000));

        assert!(matches!(
            derive_base_url(address),
            Err(AppError::Configuration(_))
        ));
    }

    #[test]
    fn derives_bracketed_base_url_for_ipv6_bind() {
        let address = SocketAddr::from((Ipv6Addr::LOCALHOST, 3000));

        assert_eq!(
            derive_base_url(address).expect("derived base URL"),
            "http://[::1]:3000"
        );
    }

    #[test]
    fn derives_base_url_for_ipv4_bind() {
        let address = SocketAddr::from(([127, 0, 0, 1], 3000));

        assert_eq!(
            derive_base_url(address).expect("derived base URL"),
            "http://127.0.0.1:3000"
        );
    }
}
