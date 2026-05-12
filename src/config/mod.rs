use std::{env, net::SocketAddr};

use crate::error::AppError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub base_url: String,
    pub database_url: String,
    pub redis_url: String,
    pub redirect_status_code: u16,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, AppError> {
        Ok(Self {
            host: env::var("APP_HOST").unwrap_or_else(|_| "127.0.0.1".to_owned()),
            port: parse_env("APP_PORT", 3000)?,
            base_url: env::var("BASE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:3000".to_owned()),
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://postgres:postgres@127.0.0.1:5432/url_shortener".to_owned()),
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_owned()),
            redirect_status_code: parse_env("REDIRECT_STATUS_CODE", 302)?,
        })
    }

    pub fn bind_address(&self) -> SocketAddr {
        format!("{}:{}", self.host, self.port)
            .parse()
            .expect("invalid bind address")
    }
}

fn parse_env<T>(key: &str, default: T) -> Result<T, AppError>
where
    T: std::str::FromStr,
{
    match env::var(key) {
        Ok(value) => value.parse().map_err(|_| {
            AppError::Configuration(format!("failed to parse environment variable `{key}`"))
        }),
        Err(env::VarError::NotPresent) => Ok(default),
        Err(_) => Err(AppError::Configuration(format!(
            "failed to read environment variable `{key}`"
        ))),
    }
}
