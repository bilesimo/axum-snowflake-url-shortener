use redis::{AsyncCommands, Client, aio::ConnectionManager};
use tracing::trace;

use crate::{configuration::RedisSettings, error::AppError};

#[derive(Clone)]
pub struct RedisUrlCache {
    connection_manager: ConnectionManager,
    key_prefix: String,
    ttl_seconds: u64,
}

impl RedisUrlCache {
    pub fn new(
        connection_manager: ConnectionManager,
        key_prefix: impl Into<String>,
        ttl_seconds: u64,
    ) -> Self {
        Self {
            connection_manager,
            key_prefix: key_prefix.into(),
            ttl_seconds,
        }
    }

    pub async fn from_settings(settings: &RedisSettings) -> Result<Self, AppError> {
        let client = Client::open(settings.connection_string())
            .map_err(|error| AppError::Configuration(format!("invalid Redis URL: {error}")))?;
        let connection_manager = ConnectionManager::new(client)
            .await
            .map_err(|error| AppError::Internal(format!("failed to connect to Redis: {error}")))?;

        Ok(Self::new(
            connection_manager,
            settings.key_prefix.clone(),
            settings.ttl_seconds,
        ))
    }

    pub fn key_for(&self, short_code: &str) -> String {
        format!("{}:code:{short_code}", self.key_prefix)
    }

    pub async fn get_long_url(&self, short_code: &str) -> Result<Option<String>, AppError> {
        let mut connection = self.connection_manager.clone();
        let key = self.key_for(short_code);
        trace!(%key, "reading URL from Redis");

        connection
            .get(key)
            .await
            .map_err(|error| AppError::Internal(format!("failed to read from Redis: {error}")))
    }

    pub async fn set_long_url(&self, short_code: &str, long_url: &str) -> Result<(), AppError> {
        let mut connection = self.connection_manager.clone();
        let key = self.key_for(short_code);
        trace!(%key, ttl_seconds = self.ttl_seconds, "writing URL to Redis with TTL");

        connection
            .set_ex(key, long_url, self.ttl_seconds)
            .await
            .map_err(|error| AppError::Internal(format!("failed to write to Redis: {error}")))
    }
}
