use async_trait::async_trait;
use redis::{AsyncCommands, Client};

use crate::{error::AppError, storage::UrlCache};

#[derive(Clone, Debug)]
pub struct RedisUrlCache {
    client: Client,
}

impl RedisUrlCache {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub fn from_url(redis_url: &str) -> Result<Self, AppError> {
        let client = Client::open(redis_url)
            .map_err(|error| AppError::Configuration(format!("invalid Redis URL: {error}")))?;

        Ok(Self::new(client))
    }

    fn key_for(short_code: &str) -> String {
        format!("shorturl:code:{short_code}")
    }
}

#[async_trait]
impl UrlCache for RedisUrlCache {
    async fn get_long_url(&self, short_code: &str) -> Result<Option<String>, AppError> {
        let mut connection = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|error| AppError::Internal(format!("failed to connect to Redis: {error}")))?;

        connection
            .get(Self::key_for(short_code))
            .await
            .map_err(|error| AppError::Internal(format!("failed to read from Redis: {error}")))
    }

    async fn set_long_url(&self, short_code: &str, long_url: &str) -> Result<(), AppError> {
        let mut connection = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|error| AppError::Internal(format!("failed to connect to Redis: {error}")))?;

        connection
            .set(Self::key_for(short_code), long_url)
            .await
            .map_err(|error| AppError::Internal(format!("failed to write to Redis: {error}")))
    }
}

#[cfg(test)]
mod tests {
    use super::RedisUrlCache;

    #[test]
    fn builds_expected_key_shape() {
        assert_eq!(RedisUrlCache::key_for("zn9edcu"), "shorturl:code:zn9edcu");
    }
}
