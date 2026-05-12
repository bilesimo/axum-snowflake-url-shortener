use redis::{AsyncCommands, Client};
use reqwest::{Client as HttpClient, Response, redirect::Policy};
use sqlx::{Connection, Executor, PgConnection, PgPool};
use url_shortener::{
    configuration::{DatabaseSettings, get_configuration},
    startup::{Application, get_connection_pool},
};
use uuid::Uuid;

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
    pub redis_client: Client,
    pub redis_key_prefix: String,
    pub http_client: HttpClient,
}

impl TestApp {
    pub async fn post_shorten(&self, long_url: &str) -> Response {
        self.http_client
            .post(format!("{}/api/v1/data/shorten", self.address))
            .json(&serde_json::json!({ "long_url": long_url }))
            .send()
            .await
            .expect("failed to execute shorten request")
    }

    pub async fn get_redirect(&self, short_code: &str) -> Response {
        self.http_client
            .get(format!("{}/{}", self.address, short_code))
            .send()
            .await
            .expect("failed to execute redirect request")
    }

    pub async fn delete_short_url_from_db(&self, short_code: &str) {
        sqlx::query("DELETE FROM short_urls WHERE short_code = $1")
            .bind(short_code)
            .execute(&self.db_pool)
            .await
            .expect("failed to delete short URL from database");
    }

    pub async fn cached_long_url(&self, short_code: &str) -> Option<String> {
        let mut connection = self
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .expect("failed to connect to Redis");

        connection
            .get(format!("{}:code:{short_code}", self.redis_key_prefix))
            .await
            .expect("failed to fetch cached long URL")
    }
}

pub async fn spawn_app() -> TestApp {
    let mut configuration = get_configuration().expect("failed to read configuration");
    configuration.database.database_name = Uuid::new_v4().to_string();
    configuration.application.port = 0;
    configuration.application.base_url = String::new();
    configuration.redis.key_prefix = format!("test-shorturl-{}", Uuid::new_v4());

    configure_database(&configuration.database).await;

    let redis_client =
        Client::open(configuration.redis.connection_string()).expect("invalid Redis URL");
    let application = Application::build(configuration.clone())
        .await
        .expect("failed to build application");
    let port = application.port();
    let _server = tokio::spawn(application.run_until_stopped());

    TestApp {
        address: format!("http://127.0.0.1:{port}"),
        db_pool: get_connection_pool(&configuration.database)
            .await
            .expect("failed to get database connection pool"),
        redis_client,
        redis_key_prefix: configuration.redis.key_prefix,
        http_client: HttpClient::builder()
            .redirect(Policy::none())
            .build()
            .expect("failed to build reqwest client"),
    }
}

async fn configure_database(config: &DatabaseSettings) {
    let mut connection = PgConnection::connect_with(&config.without_db())
        .await
        .expect("failed to connect to Postgres without database");

    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("failed to create test database");
}
