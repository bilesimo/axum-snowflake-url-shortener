use tracing_subscriber::EnvFilter;
use url_shortener::{configuration::get_configuration, startup::Application};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let configuration = get_configuration().expect("failed to read configuration");
    let application = Application::build(configuration).await?;
    application.run_until_stopped().await?;

    Ok(())
}
