use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderValue, StatusCode, header::LOCATION},
    response::Response,
};

use crate::{AppState, error::AppError};

use super::dto::{HealthResponse, ShortenRequest, ShortenResponse};

pub async fn shorten(
    State(state): State<AppState>,
    Json(request): Json<ShortenRequest>,
) -> Result<(StatusCode, Json<ShortenResponse>), AppError> {
    let short_url = state.service.create_short_url(&request.long_url).await?;
    let response = ShortenResponse {
        short_code: short_url.short_code.clone(),
        short_url: format!(
            "{}/{}",
            state.config.base_url.trim_end_matches('/'),
            short_url.short_code
        ),
        long_url: short_url.long_url,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

pub async fn redirect(
    State(state): State<AppState>,
    Path(short_code): Path<String>,
) -> Result<Response, AppError> {
    let long_url = state.service.resolve_short_code(&short_code).await?;
    let location = HeaderValue::from_str(&long_url).map_err(|error| {
        AppError::Internal(format!("failed to build redirect location header: {error}"))
    })?;

    let mut response = Response::new(axum::body::Body::empty());
    *response.status_mut() = state.config.redirect_status();
    response.headers_mut().insert(LOCATION, location);

    Ok(response)
}
