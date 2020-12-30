use std::env;
use std::fmt::Display;

use deadpool_redis::Connection;
use deadpool_redis::Pool as RedisPool;
use deadpool_redis::{cmd, Config};
use redis::RedisError;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::stats::Stats;

pub struct Cache {
    pool: Option<RedisPool>,
    ttl: i32,
}

lazy_static! {
    static ref CACHE_POOL: Cache = Cache::new();
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

    /// create a new cache object, this ignores all errors to make sure the cache doesn't break the application
    fn new() -> Self {
        let mut cache_pool = Cache::default();
        let redis_url = match env::var("REDIS_URL") {
            Ok(redis_url) => redis_url,
            Err(_e) => {
                info!("cache pool not initialising due to missing `REDIS_URL`");
                return cache_pool;
            }
        };

        let mut cfg = Config::default();
        cfg.url = Some(redis_url);

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
    pub(crate) fn is_enabled() -> bool {
        CACHE_POOL.pool.is_some()
    }

    async fn connection(&self) -> Option<Connection> {
        match self.pool.as_ref()?.get().await {
            Ok(connection) => Some(connection),
            Err(err) => {
                error!("unable to get cache connection: {}", err);
                None
            }
        }
    }

    pub(crate) async fn get<T: DeserializeOwned + CacheIdentifier, I: Display>(id: I) -> Option<T> {
        let mut conn = CACHE_POOL.connection().await?;
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

                return cache_hit;
            }
            Err(err) => {
                error!("unable to fetch {} from cache: {}", &cache_key, err);
                return None;
            }
        };
    }

    pub(crate) async fn set<T: Serialize + CacheIdentifier, I: Display>(object: &T, id: I) {
        let mut conn = match CACHE_POOL.connection().await {
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

        let res = cmd("SETEX")
            .arg(cache_key)
            .arg(CACHE_POOL.ttl)
            .arg(object_string)
            .execute_async(&mut conn)
            .await;

        if let Err(err) = res {
            error!("unable to store object in cache: {}", err);
        }
    }

    #[allow(dead_code)]
    pub(crate) async fn delete(cache_key: String) {
        let mut conn = match CACHE_POOL.connection().await {
            Some(conn) => conn,
            None => return,
        };

        let res = cmd("DEL").arg(&cache_key).execute_async(&mut conn).await;

        if let Err(err) = res {
            error!("unable to delete object from cache: {}", err);
        }
    }
}
