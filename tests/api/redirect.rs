use reqwest::StatusCode;
use serde_json::Value;

use super::helpers::spawn_app;

#[tokio::test]
async fn redirect_returns_location_header_for_existing_code() {
    let app = spawn_app().await;

    let payload: Value = app
        .post_shorten("https://example.com/redirect-me")
        .await
        .json()
        .await
        .expect("shorten response JSON");
    let short_code = payload["short_code"].as_str().expect("short code");

    let response = app.get_redirect(short_code).await;

    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(
        response
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok()),
        Some("https://example.com/redirect-me")
    );
}

#[tokio::test]
async fn redirect_uses_redis_after_the_first_database_lookup() {
    let app = spawn_app().await;

    let payload: Value = app
        .post_shorten("https://example.com/cache-me")
        .await
        .json()
        .await
        .expect("shorten response JSON");
    let short_code = payload["short_code"].as_str().expect("short code");

    let first_response = app.get_redirect(short_code).await;
    assert_eq!(first_response.status(), StatusCode::FOUND);
    assert_eq!(
        app.cached_long_url(short_code).await.as_deref(),
        Some("https://example.com/cache-me")
    );

    app.delete_short_url_from_db(short_code).await;

    let second_response = app.get_redirect(short_code).await;
    assert_eq!(second_response.status(), StatusCode::FOUND);
    assert_eq!(
        second_response
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok()),
        Some("https://example.com/cache-me")
    );
}
