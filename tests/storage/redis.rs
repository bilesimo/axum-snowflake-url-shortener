use redis::AsyncCommands;
use url_shortener::storage::redis::RedisUrlCache;

use crate::helpers::{redis_client, test_configuration};

#[tokio::test]
async fn redis_cache_sets_and_gets_long_url() {
    let configuration = test_configuration().await;
    let cache = RedisUrlCache::from_settings(&configuration.redis)
        .await
        .expect("cache");

    cache
        .set_long_url("abc123", "https://example.com/from-redis")
        .await
        .expect("set");

    let fetched = cache.get_long_url("abc123").await.expect("get");
    assert_eq!(fetched.as_deref(), Some("https://example.com/from-redis"));
}

#[tokio::test]
async fn redis_cache_sets_a_ttl() {
    let configuration = test_configuration().await;
    let cache = RedisUrlCache::from_settings(&configuration.redis)
        .await
        .expect("cache");

    cache
        .set_long_url("ttl123", "https://example.com/ttl")
        .await
        .expect("set");

    let client = redis_client(&configuration.redis);
    let mut connection = client
        .get_multiplexed_async_connection()
        .await
        .expect("failed to connect to Redis");
    let ttl: i64 = connection.ttl(cache.key_for("ttl123")).await.expect("ttl");

    assert!(ttl > 0);
    assert!(ttl <= configuration.redis.ttl_seconds as i64);
}
