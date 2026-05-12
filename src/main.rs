mod config;
mod domain;
mod error;
mod http;
mod id;
mod storage;

use std::sync::Arc;

use axum::Router;
use config::AppConfig;
use tokio::net::TcpListener;
use tracing::info;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = Arc::new(AppConfig::from_env()?);
    let app = app_router(AppState {
        config: Arc::clone(&config),
    });

    let listener = TcpListener::bind(config.bind_address()).await?;
    info!("listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await?;

    Ok(())
}

pub fn app_router(state: AppState) -> Router {
    http::router::build_router(state)
}
