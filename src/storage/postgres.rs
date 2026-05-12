use async_trait::async_trait;
use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::{
    domain::model::{NewShortUrl, ShortUrl},
    error::AppError,
    storage::UrlRepository,
};

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[derive(Clone, Debug)]
pub struct PostgresUrlRepository {
    pool: PgPool,
}

impl PostgresUrlRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn connect(database_url: &str, max_connections: u32) -> Result<Self, AppError> {
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .connect(database_url)
            .await
            .map_err(|error| AppError::Internal(format!("failed to connect to postgres: {error}")))?;

        Ok(Self::new(pool))
    }

    pub async fn run_migrations(&self) -> Result<(), AppError> {
        MIGRATOR
            .run(&self.pool)
            .await
            .map_err(|error| AppError::Internal(format!("failed to run postgres migrations: {error}")))
    }
}

#[derive(sqlx::FromRow)]
struct ShortUrlRow {
    id: i64,
    short_code: String,
    long_url: String,
    normalized_long_url: String,
}

impl From<ShortUrlRow> for ShortUrl {
    fn from(row: ShortUrlRow) -> Self {
        Self {
            id: row.id,
            short_code: row.short_code,
            long_url: row.long_url,
            normalized_long_url: row.normalized_long_url,
        }
    }
}

#[async_trait]
impl UrlRepository for PostgresUrlRepository {
    async fn find_by_short_code(&self, short_code: &str) -> Result<Option<ShortUrl>, AppError> {
        let row = sqlx::query_as::<_, ShortUrlRow>(
            r#"
            SELECT id, short_code, long_url, normalized_long_url
            FROM short_urls
            WHERE short_code = $1
            "#,
        )
        .bind(short_code)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| AppError::Internal(format!("failed to query short code: {error}")))?;

        Ok(row.map(Into::into))
    }

    async fn find_by_normalized_long_url(
        &self,
        normalized_long_url: &str,
    ) -> Result<Option<ShortUrl>, AppError> {
        let row = sqlx::query_as::<_, ShortUrlRow>(
            r#"
            SELECT id, short_code, long_url, normalized_long_url
            FROM short_urls
            WHERE normalized_long_url = $1
            "#,
        )
        .bind(normalized_long_url)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| AppError::Internal(format!("failed to query normalized long url: {error}")))?;

        Ok(row.map(Into::into))
    }

    async fn insert(&self, new_short_url: &NewShortUrl) -> Result<ShortUrl, AppError> {
        let row = sqlx::query_as::<_, ShortUrlRow>(
            r#"
            INSERT INTO short_urls (id, short_code, long_url, normalized_long_url)
            VALUES ($1, $2, $3, $4)
            RETURNING id, short_code, long_url, normalized_long_url
            "#,
        )
        .bind(new_short_url.id)
        .bind(&new_short_url.short_code)
        .bind(&new_short_url.long_url)
        .bind(&new_short_url.normalized_long_url)
        .fetch_one(&self.pool)
        .await
        .map_err(map_insert_error)?;

        Ok(row.into())
    }
}

fn map_insert_error(error: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(database_error) = &error {
        if database_error.code().as_deref() == Some("23505") {
            return AppError::Conflict(format!("short url mapping already exists: {database_error}"));
        }
    }

    AppError::Internal(format!("failed to insert short url mapping: {error}"))
}
