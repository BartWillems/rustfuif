#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate serde_derive;

use std::fmt::{Debug, Display};

use deadpool_redis::cmd;
use deadpool_redis::Connection;
use deadpool_redis::Pool as RedisPool;
use redis::RedisError;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::sync::RwLock;
use tracing::{debug, error};

mod stats;

pub use stats::Stats;

lazy_static! {
    static ref CACHE_POOL: RwLock<Cache> = RwLock::new(Cache::default());
    static ref REDIS_URL: RwLock<Option<String>> = RwLock::new(None);
}

pub struct Cache {
    pool: Option<RedisPool>,
    ttl: i32,
}

#[derive(Serialize, Debug)]
pub struct CacheStatus {
    /// is true when the redis url is set and is a valid url
    enabled: bool,
    /// is true when the cache is enabled and a connection can be retrieved
    healthy: bool,
}

pub trait CacheIdentifier {
    fn cache_key<T: Display>(id: T) -> String;
}

impl Cache {
    fn default() -> Self {
        Cache {
            pool: None,
            ttl: 3600 * 12,
        }
    }

    async fn set_url(url: String) {
        let mut cfg_url = REDIS_URL.write().await;
        *cfg_url = Some(url);
    }

    /// create a new cache object, this ignores all errors to make sure the cache doesn't break the application
    pub async fn init(url: String) -> Result<(), redis::RedisError> {
        let mut new_cache = Cache::default();

        let cfg = deadpool_redis::Config {
            url: Some(url.clone()),
            ..Default::default()
        };

        let pool = cfg.create_pool()?;
        new_cache.pool = Some(pool);

        let mut cache = CACHE_POOL.write().await;

        *cache = new_cache;
        // the url should be set to be able to reuse the cache
        Cache::set_url(url).await;
        Ok(())
    }

    /// returns true if the cache is initialized and ready for usage
    pub async fn is_enabled() -> bool {
        let cache = CACHE_POOL.read().await;
        cache.pool.is_some()
    }

    #[tracing::instrument]
    async fn connection() -> Option<Connection> {
        let cache = CACHE_POOL.read().await;

        match cache.pool.as_ref()?.get().await {
            Ok(connection) => Some(connection),
            Err(err) => {
                error!("unable to get cache connection: {}", err);
                None
            }
        }
    }

    #[tracing::instrument(name = "cache::get")]
    pub async fn get<T: DeserializeOwned + CacheIdentifier, I: Display + Debug>(
        id: I,
    ) -> Option<T> {
        let mut conn = Cache::connection().await?;
        let cache_key: String = T::cache_key(id);

        let res: Result<Vec<u8>, RedisError> =
            cmd("GET").arg(&cache_key).query_async(&mut conn).await;

        match res {
            Ok(res) => {
                let cache_hit = serde_json::from_slice::<T>(&res).ok();

                if cache_hit.is_some() {
                    Stats::cache_hit();
                    debug!("found {} in cache", &cache_key);
                } else {
                    Stats::cache_miss();
                }

                cache_hit
            }
            Err(err) => {
                error!("unable to fetch {} from cache: {}", &cache_key, err);
                None
            }
        }
    }

    #[tracing::instrument(name = "cache::set", skip(object))]
    pub async fn set<T: Serialize + CacheIdentifier, I: Display + Debug>(object: &T, id: I) {
        let mut conn = match Cache::connection().await {
            Some(conn) => conn,
            None => return,
        };

        let cache_key: String = T::cache_key(id);

        let object_string = match serde_json::to_vec(object) {
            Ok(res) => res,
            Err(err) => {
                error!("unable to serialize object for cache {}", err);
                return;
            }
        };

        let ttl = CACHE_POOL.read().await.ttl;

        let res = cmd("SETEX")
            .arg(cache_key)
            .arg(ttl)
            .arg(object_string)
            .execute_async(&mut conn)
            .await;

        if let Err(err) = res {
            error!("unable to store object in cache: {}", err);
        }
    }

    #[tracing::instrument(name = "cache::delete")]
    pub async fn delete(cache_key: String) {
        let mut conn = match Cache::connection().await {
            Some(conn) => conn,
            None => return,
        };

        let res = cmd("DEL").arg(&cache_key).execute_async(&mut conn).await;

        if let Err(err) = res {
            error!("unable to delete object from cache: {}", err);
        }
    }

    pub async fn disable_cache() {
        let mut cache = CACHE_POOL.write().await;

        cache.pool = None;
    }

    pub async fn enable_cache() -> Result<(), RedisError> {
        let url = REDIS_URL.read().await;

        let url: String = (*url
            .clone()
            .expect("the redis cache should be initialized before"))
        .to_string();

        Cache::init(url).await
    }

    pub async fn status() -> CacheStatus {
        let enabled = Cache::is_enabled().await;
        let mut healthy = true;
        if enabled {
            healthy = Cache::connection().await.is_some();
        }
        CacheStatus { enabled, healthy }
    }
}
