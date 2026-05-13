use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;
use tracing::error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("configuration error: {0}")]
    Configuration(String),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("resource not found")]
    NotFound,
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

impl AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Configuration(_) | Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Validation(_) => StatusCode::BAD_REQUEST,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
        }
    }

    fn client_message(&self) -> String {
        match self {
            Self::Configuration(_) | Self::Internal(_) => "internal server error".to_owned(),
            _ => self.to_string(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        if matches!(self, Self::Configuration(_) | Self::Internal(_)) {
            error!(error = %self, "request failed");
        }

        let status = self.status_code();
        let body = Json(ErrorResponse {
            error: self.client_message(),
        });

        (status, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use axum::{body, response::IntoResponse};

    use super::AppError;

    #[tokio::test]
    async fn internal_errors_are_sanitized_in_http_responses() {
        let response =
            AppError::Internal("database timeout: postgres://secret".to_owned()).into_response();
        let body = body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let payload = String::from_utf8(body.to_vec()).expect("utf8 body");

        assert_eq!(payload, r#"{"error":"internal server error"}"#);
    }
}
