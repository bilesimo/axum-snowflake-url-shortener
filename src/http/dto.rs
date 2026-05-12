use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct HealthResponse {
    pub status: &'static str,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct ShortenRequest {
    pub long_url: String,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct ShortenResponse {
    pub short_code: String,
    pub short_url: String,
    pub long_url: String,
}
