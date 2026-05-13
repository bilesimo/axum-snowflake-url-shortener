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
