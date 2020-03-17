use r2d2;
use redis::Client;
use url::Url;

type Pool = r2d2::Pool<Client>;
pub type CacheConnection = r2d2::PooledConnection<Client>;

pub struct Cache {
    pool: Pool,
}

impl Cache {
    pub fn init(redis_url: &Url) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Initializing Cache");
        let client = redis::Client::open(redis_url.to_string())?;

        let pool = Pool::new(client)?;

        Ok(Cache { pool })
    }

    pub fn connection(&self) -> Result<CacheConnection, r2d2::Error> {
        self.pool.get()
    }
}
