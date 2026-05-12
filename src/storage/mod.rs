use async_trait::async_trait;

use crate::{domain::model::{NewShortUrl, ShortUrl}, error::AppError};

pub mod postgres;
pub mod redis;

#[async_trait]
pub trait UrlRepository: Send + Sync {
    async fn find_by_short_code(&self, short_code: &str) -> Result<Option<ShortUrl>, AppError>;
    async fn find_by_normalized_long_url(
        &self,
        normalized_long_url: &str,
    ) -> Result<Option<ShortUrl>, AppError>;
    async fn insert(&self, new_short_url: &NewShortUrl) -> Result<ShortUrl, AppError>;
}

#[async_trait]
pub trait UrlCache: Send + Sync {
    async fn get_long_url(&self, short_code: &str) -> Result<Option<String>, AppError>;
    async fn set_long_url(&self, short_code: &str, long_url: &str) -> Result<(), AppError>;
}
