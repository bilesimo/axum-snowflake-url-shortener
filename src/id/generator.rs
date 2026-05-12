use std::sync::{
    Arc,
    atomic::{AtomicI64, Ordering},
};

use async_trait::async_trait;
use sqlx::PgPool;

use crate::error::AppError;

#[async_trait]
pub trait IdGenerator: Send + Sync {
    async fn next_id(&self) -> Result<i64, AppError>;
}

#[derive(Debug)]
pub struct InMemoryIdGenerator {
    next: AtomicI64,
}

impl InMemoryIdGenerator {
    pub fn new(start_from: i64) -> Self {
        Self {
            next: AtomicI64::new(start_from),
        }
    }
}

#[async_trait]
impl IdGenerator for InMemoryIdGenerator {
    async fn next_id(&self) -> Result<i64, AppError> {
        Ok(self.next.fetch_add(1, Ordering::SeqCst))
    }
}

#[derive(Clone, Debug)]
pub struct SequenceIdGenerator {
    pool: PgPool,
    sequence_name: Arc<str>,
}

impl SequenceIdGenerator {
    pub fn new(pool: PgPool, sequence_name: impl Into<Arc<str>>) -> Self {
        Self {
            pool,
            sequence_name: sequence_name.into(),
        }
    }
}

#[async_trait]
impl IdGenerator for SequenceIdGenerator {
    async fn next_id(&self) -> Result<i64, AppError> {
        let row: (i64,) = sqlx::query_as("SELECT nextval($1::regclass)")
            .bind(self.sequence_name.as_ref())
            .fetch_one(&self.pool)
            .await
            .map_err(|error| AppError::Internal(format!("failed to fetch next sequence value: {error}")))?;

        Ok(row.0)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tokio::task::JoinSet;

    use super::{IdGenerator, InMemoryIdGenerator};

    #[tokio::test]
    async fn in_memory_generator_returns_unique_ids() {
        let generator = InMemoryIdGenerator::new(42);

        assert_eq!(generator.next_id().await.expect("id"), 42);
        assert_eq!(generator.next_id().await.expect("id"), 43);
        assert_eq!(generator.next_id().await.expect("id"), 44);
    }

    #[tokio::test]
    async fn in_memory_generator_is_safe_under_concurrency() {
        let generator = Arc::new(InMemoryIdGenerator::new(100));
        let mut tasks = JoinSet::new();

        for _ in 0..32 {
            let generator = Arc::clone(&generator);
            tasks.spawn(async move { generator.next_id().await.expect("id") });
        }

        let mut ids = Vec::new();

        while let Some(result) = tasks.join_next().await {
            ids.push(result.expect("task"));
        }

        ids.sort_unstable();
        ids.dedup();

        assert_eq!(ids.len(), 32);
        assert_eq!(ids.first().copied(), Some(100));
        assert_eq!(ids.last().copied(), Some(131));
    }
}
