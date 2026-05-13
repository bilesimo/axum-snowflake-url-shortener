use reqwest::StatusCode;
use serde_json::Value;

use super::helpers::spawn_app;

#[tokio::test]
async fn shorten_creates_a_mapping_and_persists_it() {
    let app = spawn_app().await;

    let response = app.post_shorten("https://example.com/articles/123").await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let payload: Value = response.json().await.expect("invalid JSON response");
    let short_code = payload["short_code"]
        .as_str()
        .expect("short code should be present");
    assert!(!short_code.is_empty());
    assert_eq!(payload["long_url"], "https://example.com/articles/123");
    assert_eq!(
        payload["short_url"],
        format!("{}/{}", app.address, short_code)
    );

    let row = sqlx::query_as::<_, (String, String)>(
        r#"SELECT short_code, long_url FROM short_urls WHERE short_code = $1"#,
    )
    .bind(short_code)
    .fetch_one(&app.db_pool)
    .await
    .expect("failed to fetch inserted row");

    assert_eq!(row.0, short_code);
    assert_eq!(row.1, "https://example.com/articles/123");
}

#[tokio::test]
async fn shorten_deduplicates_repeated_long_urls() {
    let app = spawn_app().await;

    let first: Value = app
        .post_shorten("https://example.com/same-url")
        .await
        .json()
        .await
        .expect("first response JSON");
    let second: Value = app
        .post_shorten("https://example.com/same-url")
        .await
        .json()
        .await
        .expect("second response JSON");

    assert_eq!(first["short_code"], second["short_code"]);

    let count =
        sqlx::query_scalar::<_, i64>(r#"SELECT COUNT(*) FROM short_urls WHERE long_url = $1"#)
            .bind("https://example.com/same-url")
            .fetch_one(&app.db_pool)
            .await
            .expect("failed to count rows");

    assert_eq!(count, 1);
}
