use reqwest::StatusCode;

use super::helpers::spawn_app;

#[tokio::test]
async fn health_check_works() {
    let app = spawn_app().await;

    let response = app
        .http_client
        .get(format!("{}/health", app.address))
        .send()
        .await
        .expect("failed to execute health check request");

    assert_eq!(response.status(), StatusCode::OK);
}
