use tracing::{debug, info, instrument, trace};
use url::Url;

use crate::{
    domain::model::ShortUrl,
    error::AppError,
    id::{base62::encode_base62, generator::SnowflakeIdGenerator},
    storage::{postgres::PostgresUrlRepository, redis::RedisUrlCache},
};

#[derive(Clone)]
pub struct UrlShortenerService {
    repository: PostgresUrlRepository,
    cache: RedisUrlCache,
    id_generator: SnowflakeIdGenerator,
}

impl UrlShortenerService {
    pub fn new(
        repository: PostgresUrlRepository,
        cache: RedisUrlCache,
        id_generator: SnowflakeIdGenerator,
    ) -> Self {
        Self {
            repository,
            cache,
            id_generator,
        }
    }

    #[instrument(skip(self), fields(long_url))]
    pub async fn create_short_url(&self, long_url: &str) -> Result<ShortUrl, AppError> {
        let normalized_long_url = normalize_url(long_url)?;

        if let Some(existing) = self
            .repository
            .find_by_long_url(&normalized_long_url)
            .await?
        {
            debug!(
                short_code = %existing.short_code,
                normalized_long_url = %normalized_long_url,
                "deduplicated normalized long URL using existing mapping"
            );
            return Ok(existing);
        }

        let id = self.id_generator.next_id().await?;
        let short_url = ShortUrl {
            id,
            short_code: encode_base62(id as u64),
            long_url: normalized_long_url,
        };

        let inserted = self.repository.insert(&short_url).await?;
        info!(id = inserted.id, short_code = %inserted.short_code, "created short URL mapping");
        Ok(inserted)
    }

    #[instrument(skip(self), fields(short_code))]
    pub async fn resolve_short_code(&self, short_code: &str) -> Result<String, AppError> {
        if let Some(long_url) = self.cache.get_long_url(short_code).await? {
            trace!("cache hit for short code");
            return Ok(long_url);
        }

        trace!("cache miss for short code");
        let short_url = self
            .repository
            .find_by_short_code(short_code)
            .await?
            .ok_or(AppError::NotFound)?;

        self.cache
            .set_long_url(short_code, &short_url.long_url)
            .await?;
        debug!("backfilled cache for short code");

        Ok(short_url.long_url)
    }
}

pub fn normalize_url(input: &str) -> Result<String, AppError> {
    let mut parsed =
        Url::parse(input).map_err(|error| AppError::Validation(format!("invalid URL: {error}")))?;

    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(AppError::Validation(
            "URL scheme must be http or https".to_owned(),
        ));
    }

    let remove_default_port = matches!(
        (parsed.scheme(), parsed.port()),
        ("http", Some(80)) | ("https", Some(443))
    );
    if remove_default_port {
        parsed
            .set_port(None)
            .map_err(|()| AppError::Validation("failed to normalize URL port".to_owned()))?;
    }

    Ok(parsed.to_string())
}

#[cfg(test)]
mod tests {
    use super::normalize_url;
    use crate::error::AppError;

    #[test]
    fn normalize_url_lowercases_host_and_removes_default_port() {
        let normalized = normalize_url("HTTPS://Example.COM:443/articles/123?draft=true")
            .expect("normalized URL");

        assert_eq!(normalized, "https://example.com/articles/123?draft=true");
    }

    #[test]
    fn normalize_url_keeps_fragment_when_present() {
        let normalized =
            normalize_url("https://example.com/articles/123#summary").expect("normalized URL");

        assert_eq!(normalized, "https://example.com/articles/123#summary");
    }

    #[test]
    fn normalize_url_rejects_non_http_urls() {
        let error = normalize_url("ftp://example.com/file").expect_err("validation error");
        assert!(matches!(error, AppError::Validation(_)));
    }
}
