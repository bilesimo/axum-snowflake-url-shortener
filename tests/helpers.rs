use redis::Client;
use sqlx::{Connection, Executor, PgConnection};
use url_shortener::configuration::{DatabaseSettings, RedisSettings, Settings, get_configuration};
use uuid::Uuid;

pub async fn test_configuration() -> Settings {
    let mut configuration = get_configuration().expect("failed to read configuration");
    configuration.database.database_name = Uuid::new_v4().to_string();
    configuration.application.port = 0;
    configuration.application.base_url = String::new();
    configuration.redis.key_prefix = format!("test-shorturl-{}", Uuid::new_v4());

    configure_database(&configuration.database).await;

    configuration
}

pub async fn configure_database(config: &DatabaseSettings) {
    let mut connection = PgConnection::connect_with(&config.without_db())
        .await
        .expect("failed to connect to Postgres without database");

    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("failed to create test database");
}

pub fn redis_client(config: &RedisSettings) -> Client {
    Client::open(config.connection_string()).expect("invalid Redis URL")
}
