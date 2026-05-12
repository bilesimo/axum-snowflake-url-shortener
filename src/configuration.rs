use std::{net::SocketAddr, path::Path};

use axum::http::StatusCode;
use serde::Deserialize;
use sqlx::postgres::{PgConnectOptions, PgSslMode};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Settings {
    pub application: ApplicationSettings,
    pub database: DatabaseSettings,
    pub redis: RedisSettings,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ApplicationSettings {
    pub host: String,
    pub port: u16,
    pub base_url: String,
    pub redirect_status_code: u16,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct DatabaseSettings {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database_name: String,
    pub require_ssl: bool,
    pub max_connections: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct RedisSettings {
    pub host: String,
    pub port: u16,
    pub key_prefix: String,
}

impl ApplicationSettings {
    pub fn address_string(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn bind_address(&self) -> SocketAddr {
        self.address_string()
            .parse()
            .expect("invalid bind address")
    }

    pub fn redirect_status(&self) -> StatusCode {
        match self.redirect_status_code {
            301 => StatusCode::MOVED_PERMANENTLY,
            _ => StatusCode::FOUND,
        }
    }
}

impl DatabaseSettings {
    pub fn without_db(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            PgSslMode::Prefer
        };

        PgConnectOptions::new()
            .host(&self.host)
            .port(self.port)
            .username(&self.username)
            .password(&self.password)
            .ssl_mode(ssl_mode)
    }

    pub fn with_db(&self) -> PgConnectOptions {
        self.without_db().database(&self.database_name)
    }
}

impl RedisSettings {
    pub fn connection_string(&self) -> String {
        format!("redis://{}:{}", self.host, self.port)
    }
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let config_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("config.toml");

    config::Config::builder()
        .add_source(config::File::from(config_path))
        .add_source(
            config::Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__"),
        )
        .build()?
        .try_deserialize::<Settings>()
}
