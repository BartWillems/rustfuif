use crate::errors::ServiceError;
use redis::{Client, Commands, ConnectionLike};
use std::env;

type Pool = r2d2::Pool<Client>;
pub type CacheConnection = r2d2::PooledConnection<Client>;

lazy_static! {
    static ref POOL: Pool = {
        let redis_url = env::var("REDIS_URL").expect("Redis url not set");
        let client = redis::Client::open(redis_url).expect("Failed to create redis client");
        Pool::new(client).expect("Failed to create redis pool")
    };
}

pub fn init() {
    info!("initializing redis cache");
    lazy_static::initialize(&POOL);
    let mut conn = connection().expect("failed to get redis connection");
    assert_eq!(
        true,
        conn.check_connection(),
        "Redis connection check failed"
    );
}

pub fn connection() -> Result<CacheConnection, ServiceError> {
    POOL.get().map_err(|e| {
        error!("unable to fetch redis connection: {}", e);
        ServiceError::InternalServerError
    })
}

pub trait Cache {
    fn cache_key<T: std::fmt::Display>(id: T) -> String;
}

pub fn find<T: serde::de::DeserializeOwned + Cache, I: std::fmt::Display>(
    id: I,
) -> Result<Option<T>, ServiceError> {
    let cache_key: String = T::cache_key(id);
    let mut cache = connection()?;
    let res: Vec<u8> = cache.get(&cache_key)?;

    match serde_json::from_slice::<T>(&res).ok() {
        Some(res) => {
            debug!("found {} in cache", cache_key);
            Ok(Some(res))
        }
        None => Ok(None),
    }
}

pub fn set<T: serde::Serialize + Cache, I: std::fmt::Display>(
    arg: &T,
    id: I,
) -> Result<(), ServiceError> {
    let cache_key: String = T::cache_key(id);
    let mut cache = connection()?;
    if let Ok(res) = serde_json::to_vec(arg) {
        let _: () = cache.set_ex(&cache_key, res, 3600)?;
    }
    Ok(())
}

pub fn delete(cache_key: String) -> Result<(), ServiceError> {
    let mut cache = connection()?;
    let _: () = cache.del(cache_key)?;
    Ok(())
}
