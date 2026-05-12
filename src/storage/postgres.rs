use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::{domain::model::ShortUrl, error::AppError};

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[derive(Clone, Debug)]
pub struct PostgresUrlRepository {
    pool: PgPool,
}

#[derive(sqlx::FromRow)]
struct ShortUrlRow {
    id: i64,
    short_code: String,
    long_url: String,
}

impl From<ShortUrlRow> for ShortUrl {
    fn from(row: ShortUrlRow) -> Self {
        Self {
            id: row.id,
            short_code: row.short_code,
            long_url: row.long_url,
        }
    }
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
            .map_err(|error| AppError::Internal(format!("failed to connect to Postgres: {error}")))?;

        Ok(Self::new(pool))
    }

    pub async fn run_migrations(&self) -> Result<(), AppError> {
        MIGRATOR
            .run(&self.pool)
            .await
            .map_err(|error| AppError::Internal(format!("failed to run postgres migrations: {error}")))
    }

    pub async fn find_by_short_code(&self, short_code: &str) -> Result<Option<ShortUrl>, AppError> {
        let row = sqlx::query_as::<_, ShortUrlRow>(
            r#"
            SELECT id, short_code, long_url
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

    pub async fn find_by_long_url(&self, long_url: &str) -> Result<Option<ShortUrl>, AppError> {
        let row = sqlx::query_as::<_, ShortUrlRow>(
            r#"
            SELECT id, short_code, long_url
            FROM short_urls
            WHERE long_url = $1
            ORDER BY created_at ASC
            LIMIT 1
            "#,
        )
        .bind(long_url)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| AppError::Internal(format!("failed to query long URL: {error}")))?;

        Ok(row.map(Into::into))
    }

    pub async fn insert(&self, short_url: &ShortUrl) -> Result<ShortUrl, AppError> {
        let row = sqlx::query_as::<_, ShortUrlRow>(
            r#"
            INSERT INTO short_urls (id, short_code, long_url)
            VALUES ($1, $2, $3)
            RETURNING id, short_code, long_url
            "#,
        )
        .bind(short_url.id)
        .bind(&short_url.short_code)
        .bind(&short_url.long_url)
        .fetch_one(&self.pool)
        .await
        .map_err(map_insert_error)?;

        Ok(row.into())
    }
}

fn map_insert_error(error: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(database_error) = &error {
        if database_error.code().as_deref() == Some("23505") {
            return AppError::Conflict(format!("short code already exists: {database_error}"));
        }
    }

    AppError::Internal(format!("failed to insert short URL mapping: {error}"))
}
