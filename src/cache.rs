use std::fmt::{Debug, Display};

use deadpool_redis::cmd;
use deadpool_redis::Connection;
use deadpool_redis::Pool as RedisPool;
use redis::RedisError;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::sync::RwLock;

use crate::config::Config;
use crate::stats::Stats;

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

lazy_static! {
    static ref CACHE_POOL: RwLock<Cache> = RwLock::new(Cache::new());
}

impl Cache {
    fn default() -> Self {
        Cache {
            pool: None,
            ttl: 3600 * 12,
        }
    }

    /// create a new cache object, this ignores all errors to make sure the cache doesn't break the application
    fn new() -> Self {
        info!("creating cache pool");
        let mut cache_pool = Cache::default();
        let redis_url = match Config::redis_url() {
            Some(redis_url) => redis_url,
            None => {
                info!("cache pool not initialising due to missing `REDIS_URL`");
                return cache_pool;
            }
        };

        let cfg = deadpool_redis::Config {
            url: Some(redis_url.to_owned()),
            ..Default::default()
        };

        match cfg.create_pool() {
            Ok(pool) => {
                cache_pool.pool = Some(pool);
            }
            Err(err) => {
                error!("unable to initiate cache pool: {}", err);
            }
        };

        cache_pool
    }

    pub(crate) fn init() {
        info!("initializing redis cache");
        lazy_static::initialize(&CACHE_POOL);
    }

    /// returns true if the cache is initialized and ready for usage
    pub(crate) async fn is_enabled() -> bool {
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

    /// Get the key for a certain cache entry
    fn key<T, Id: Display>(id: Id) -> String {
        format!("{}.{}", std::any::type_name::<T>(), id)
    }

    #[tracing::instrument(name = "cache::get")]
    pub(crate) async fn get<T: DeserializeOwned, Id: Display + Debug>(id: Id) -> Option<T> {
        let mut conn = Cache::connection().await?;

        let key = Cache::key::<T, _>(id);

        let res: Result<Vec<u8>, RedisError> = cmd("GET").arg(&key).query_async(&mut conn).await;

        match res {
            Ok(res) => {
                let cache_hit = serde_json::from_slice::<T>(&res).ok();

                if cache_hit.is_some() {
                    Stats::cache_hit();
                    debug!("found {} in cache", &key);
                } else {
                    Stats::cache_miss();
                }

                cache_hit
            }
            Err(err) => {
                error!("unable to fetch {} from cache: {}", &key, err);
                None
            }
        }
    }

    #[tracing::instrument(name = "cache::set", skip(object))]
    pub(crate) async fn set<T: Serialize, Id: Display + Debug>(object: &T, id: Id) {
        let mut conn = match Cache::connection().await {
            Some(conn) => conn,
            None => return,
        };

        let key = Cache::key::<T, _>(id);

        let object_string = match serde_json::to_vec(object) {
            Ok(res) => res,
            Err(err) => {
                error!("unable to serialize object for cache {}", err);
                return;
            }
        };

        let ttl = CACHE_POOL.read().await.ttl;

        let res = cmd("SETEX")
            .arg(key)
            .arg(ttl)
            .arg(object_string)
            .execute_async(&mut conn)
            .await;

        if let Err(err) = res {
            error!("unable to store object in cache: {}", err);
        }
    }

    #[allow(dead_code)]
    #[tracing::instrument(name = "cache::delete")]
    pub(crate) async fn delete(cache_key: String) {
        let mut conn = match Cache::connection().await {
            Some(conn) => conn,
            None => return,
        };

        let res = cmd("DEL").arg(&cache_key).execute_async(&mut conn).await;

        if let Err(err) = res {
            error!("unable to delete object from cache: {}", err);
        }
    }

    pub(crate) async fn disable_cache() {
        let mut cache = CACHE_POOL.write().await;

        cache.pool = None;
    }

    pub(crate) async fn enable_cache() {
        let mut cache = CACHE_POOL.write().await;

        *cache = Cache::new();
    }

    pub(crate) async fn status() -> CacheStatus {
        let enabled = Cache::is_enabled().await;
        let mut healthy = true;
        if enabled {
            healthy = Cache::connection().await.is_some();
        }
        CacheStatus { enabled, healthy }
    }
}
