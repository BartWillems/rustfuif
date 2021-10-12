use std::sync::atomic::{AtomicU64, Ordering};

use validator::Validate;

#[derive(Deserialize, Debug, Validate)]
pub struct Config {
    database_url: String,
    api_host: Option<String>,
    api_port: Option<usize>,
    #[validate(length(min = 32))]
    session_private_key: String,
    redis_url: Option<String>,
    sentry_dsn: Option<String>,
    /// the interval in seconds between price updates
    #[serde(default = "default_interval")]
    price_update_interval: AtomicU64,
    /// defaults to localhost, which shouldn't cause issues if you're using udp
    opentelemetry_endpoint: Option<String>,
}

fn default_interval() -> AtomicU64 {
    AtomicU64::new(120)
}

lazy_static! {
    static ref CONFIG: Config = match envy::from_env::<Config>() {
        Ok(config) => {
            match config.validate() {
                Ok(()) => config,
                Err(e) => panic!("invalid environment variable: {}", e),
            }
        }
        Err(error) => panic!("Missing or incorrect environment variable: {}", error),
    };
}

impl Config {
    pub fn database_url() -> &'static str {
        CONFIG.database_url.as_ref()
    }

    pub fn api_host() -> &'static str {
        match &CONFIG.api_host {
            Some(host) => host.as_ref(),
            None => "localhost",
        }
    }

    pub fn api_port() -> usize {
        CONFIG.api_port.unwrap_or(8080)
    }

    pub fn session_private_key() -> &'static str {
        CONFIG.session_private_key.as_ref()
    }

    pub fn redis_url() -> Option<&'static str> {
        CONFIG.redis_url.as_ref().map(|url| url.as_ref())
    }

    pub fn sentry_dsn() -> Option<&'static str> {
        CONFIG.sentry_dsn.as_ref().map(|dsn| dsn.as_ref())
    }

    pub fn price_update_interval() -> u64 {
        CONFIG.price_update_interval.load(Ordering::SeqCst)
    }

    pub fn set_price_update_interval(interval: u64) {
        CONFIG
            .price_update_interval
            .store(interval, Ordering::SeqCst)
    }

    pub fn opentelemetry_endpoint() -> &'static str {
        match &CONFIG.opentelemetry_endpoint {
            Some(endpoint) => endpoint.as_ref(),
            None => "127.0.0.1:6831",
        }
    }
}
