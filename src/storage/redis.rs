use redis::{AsyncCommands, Client};

use crate::{configuration::RedisSettings, error::AppError};

#[derive(Clone, Debug)]
pub struct RedisUrlCache {
    client: Client,
    key_prefix: String,
}

impl RedisUrlCache {
    pub fn new(client: Client, key_prefix: impl Into<String>) -> Self {
        Self {
            client,
            key_prefix: key_prefix.into(),
        }
    }

    pub fn from_settings(settings: &RedisSettings) -> Result<Self, AppError> {
        let client = Client::open(settings.connection_string())
            .map_err(|error| AppError::Configuration(format!("invalid Redis URL: {error}")))?;

        Ok(Self::new(client, settings.key_prefix.clone()))
    }

    pub fn key_for(&self, short_code: &str) -> String {
        format!("{}:code:{short_code}", self.key_prefix)
    }

    pub async fn get_long_url(&self, short_code: &str) -> Result<Option<String>, AppError> {
        let mut connection = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|error| AppError::Internal(format!("failed to connect to Redis: {error}")))?;

        connection
            .get(self.key_for(short_code))
            .await
            .map_err(|error| AppError::Internal(format!("failed to read from Redis: {error}")))
    }

    pub async fn set_long_url(&self, short_code: &str, long_url: &str) -> Result<(), AppError> {
        let mut connection = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|error| AppError::Internal(format!("failed to connect to Redis: {error}")))?;

        connection
            .set(self.key_for(short_code), long_url)
            .await
            .map_err(|error| AppError::Internal(format!("failed to write to Redis: {error}")))
    }
}
