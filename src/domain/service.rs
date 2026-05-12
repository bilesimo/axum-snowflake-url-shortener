use std::sync::Arc;
use url::Url;

use crate::{
    domain::model::{NewShortUrl, ShortUrl},
    error::AppError,
    id::{base62::encode_base62, generator::IdGenerator},
    storage::{UrlCache, UrlRepository},
};

#[derive(Clone)]
pub struct UrlShortenerService {
    repository: Arc<dyn UrlRepository>,
    cache: Arc<dyn UrlCache>,
    id_generator: Arc<dyn IdGenerator>,
}

impl UrlShortenerService {
    pub fn new(
        repository: Arc<dyn UrlRepository>,
        cache: Arc<dyn UrlCache>,
        id_generator: Arc<dyn IdGenerator>,
    ) -> Self {
        Self {
            repository,
            cache,
            id_generator,
        }
    }

    pub async fn resolve_short_code(&self, short_code: &str) -> Result<String, AppError> {
        if let Some(long_url) = self.cache.get_long_url(short_code).await? {
            return Ok(long_url);
        }

        let short_url = self
            .repository
            .find_by_short_code(short_code)
            .await?
            .ok_or(AppError::NotFound)?;

        self.cache
            .set_long_url(short_code, &short_url.long_url)
            .await?;

        Ok(short_url.long_url)
    }

    pub async fn create_short_url(&self, long_url: &str) -> Result<ShortUrl, AppError> {
        let normalized_long_url = normalize_url(long_url)?;

        if let Some(existing) = self
            .repository
            .find_by_normalized_long_url(&normalized_long_url)
            .await?
        {
            return Ok(existing);
        }

        let id = self.id_generator.next_id().await?;
        let short_code = encode_base62(id as u64);

        let new_short_url = NewShortUrl {
            id,
            short_code,
            long_url: long_url.to_owned(),
            normalized_long_url,
        };

        self.repository.insert(&new_short_url).await
    }
}

pub fn normalize_url(input: &str) -> Result<String, AppError> {
    let mut parsed = Url::parse(input)
        .map_err(|error| AppError::Validation(format!("invalid URL: {error}")))?;

    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(AppError::Validation(
            "URL scheme must be http or https".to_owned(),
        ));
    }

    parsed.set_fragment(None);

    Ok(parsed.to_string())
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use super::{UrlShortenerService, normalize_url};
    use crate::{
        domain::model::{NewShortUrl, ShortUrl},
        error::AppError,
        id::generator::{IdGenerator, InMemoryIdGenerator},
        storage::{UrlCache, UrlRepository},
    };
    use async_trait::async_trait;
    use tokio::sync::Mutex;

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

    fn make_service(
        repository: Arc<dyn UrlRepository>,
        cache: Arc<dyn UrlCache>,
        id_generator: Arc<dyn IdGenerator>,
    ) -> UrlShortenerService {
        UrlShortenerService::new(repository, cache, id_generator)
    }

    #[tokio::test]
    async fn resolve_short_code_hits_cache_first() {
        let repository = Arc::new(FakeRepository::default());
        let cache = Arc::new(FakeCache::default());
        cache
            .set_long_url("abc123", "https://example.com/from-cache")
            .await
            .expect("cache insert");

        let service = make_service(
            repository,
            cache,
            Arc::new(InMemoryIdGenerator::new(1)),
        );

        let long_url = service
            .resolve_short_code("abc123")
            .await
            .expect("resolved");

        assert_eq!(long_url, "https://example.com/from-cache");
    }

    #[tokio::test]
    async fn resolve_short_code_backfills_cache_after_repository_miss() {
        let repository = Arc::new(FakeRepository::default());
        let cache = Arc::new(FakeCache::default());

        repository
            .insert(&NewShortUrl {
                id: 1,
                short_code: "zn9edcu".to_owned(),
                long_url: "https://example.com/from-db".to_owned(),
                normalized_long_url: "https://example.com/from-db".to_owned(),
            })
            .await
            .expect("repository insert");

        let service = make_service(
            repository,
            Arc::clone(&cache) as Arc<dyn UrlCache>,
            Arc::new(InMemoryIdGenerator::new(1)),
        );

        let long_url = service
            .resolve_short_code("zn9edcu")
            .await
            .expect("resolved");

        assert_eq!(long_url, "https://example.com/from-db");
        assert_eq!(
            cache
                .get_long_url("zn9edcu")
                .await
                .expect("cache read")
                .as_deref(),
            Some("https://example.com/from-db")
        );
    }

    #[tokio::test]
    async fn create_short_url_deduplicates_by_normalized_long_url() {
        let repository = Arc::new(FakeRepository::default());
        let cache = Arc::new(FakeCache::default());
        let service = make_service(
            Arc::clone(&repository) as Arc<dyn UrlRepository>,
            cache,
            Arc::new(InMemoryIdGenerator::new(100)),
        );

        let first = service
            .create_short_url("https://example.com/hello")
            .await
            .expect("first insert");
        let second = service
            .create_short_url("https://example.com/hello")
            .await
            .expect("deduplicated");

        assert_eq!(first, second);
        assert_eq!(first.short_code, "1C");
    }

    #[test]
    fn normalize_url_rejects_non_http_schemes() {
        let error = normalize_url("ftp://example.com").expect_err("invalid");
        assert!(matches!(error, AppError::Validation(_)));
    }
}
