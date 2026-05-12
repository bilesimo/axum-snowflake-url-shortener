use sqlx::PgPool;

use crate::error::AppError;

#[derive(Clone, Debug)]
pub struct SequenceIdGenerator {
    pool: PgPool,
    sequence_name: &'static str,
}

impl SequenceIdGenerator {
    pub fn new(pool: PgPool, sequence_name: &'static str) -> Self {
        Self {
            pool,
            sequence_name,
        }
    }

    pub async fn next_id(&self) -> Result<i64, AppError> {
        let row: (i64,) = sqlx::query_as("SELECT nextval($1::regclass)")
            .bind(self.sequence_name)
            .fetch_one(&self.pool)
            .await
            .map_err(|error| {
                AppError::Internal(format!("failed to fetch next sequence value: {error}"))
            })?;

        Ok(row.0)
    }
}
