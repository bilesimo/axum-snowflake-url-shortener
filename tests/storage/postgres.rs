use url_shortener::{
    domain::model::ShortUrl, startup::get_connection_pool, storage::postgres::PostgresUrlRepository,
};

use crate::helpers::test_configuration;

#[tokio::test]
async fn postgres_repository_inserts_and_finds_by_short_code() {
    let configuration = test_configuration().await;
    let pool = get_connection_pool(&configuration.database)
        .await
        .expect("failed to connect to Postgres");
    let repository = PostgresUrlRepository::new(pool);
    repository.run_migrations().await.expect("migrations");

    let inserted = repository
        .insert(&ShortUrl {
            id: 123,
            short_code: "abc123".to_owned(),
            long_url: "https://example.com/repository".to_owned(),
        })
        .await
        .expect("insert");

    let fetched = repository
        .find_by_short_code("abc123")
        .await
        .expect("query")
        .expect("row");

    assert_eq!(fetched, inserted);
}

#[tokio::test]
async fn postgres_repository_finds_by_long_url() {
    let configuration = test_configuration().await;
    let pool = get_connection_pool(&configuration.database)
        .await
        .expect("failed to connect to Postgres");
    let repository = PostgresUrlRepository::new(pool);
    repository.run_migrations().await.expect("migrations");

    repository
        .insert(&ShortUrl {
            id: 456,
            short_code: "def456".to_owned(),
            long_url: "https://example.com/by-long-url".to_owned(),
        })
        .await
        .expect("insert");

    let fetched = repository
        .find_by_long_url("https://example.com/by-long-url")
        .await
        .expect("query")
        .expect("row");

    assert_eq!(fetched.short_code, "def456");
    assert_eq!(fetched.id, 456);
}

#[tokio::test]
async fn postgres_repository_returns_existing_row_on_long_url_conflict() {
    let configuration = test_configuration().await;
    let pool = get_connection_pool(&configuration.database)
        .await
        .expect("failed to connect to Postgres");
    let repository = PostgresUrlRepository::new(pool);
    repository.run_migrations().await.expect("migrations");

    let first = repository
        .insert(&ShortUrl {
            id: 100,
            short_code: "first".to_owned(),
            long_url: "https://example.com/conflict".to_owned(),
        })
        .await
        .expect("first insert");

    let second = repository
        .insert(&ShortUrl {
            id: 101,
            short_code: "second".to_owned(),
            long_url: "https://example.com/conflict".to_owned(),
        })
        .await
        .expect("second insert");

    assert_eq!(second, first);

    let count =
        sqlx::query_scalar::<_, i64>(r#"SELECT COUNT(*) FROM short_urls WHERE long_url = $1"#)
            .bind("https://example.com/conflict")
            .fetch_one(repository.pool())
            .await
            .expect("count");

    assert_eq!(count, 1);
}
