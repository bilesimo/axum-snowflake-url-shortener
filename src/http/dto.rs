use serde::Serialize;

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct HealthResponse {
    pub status: &'static str,
}
