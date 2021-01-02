use std::time::Duration;

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
    price_update_interval: Option<u64>,
    /// defaults to localhost, which shouldn't cause issues if you're using udp
    opentelemetry_endpoint: Option<String>,
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
        match &CONFIG.redis_url {
            Some(url) => Some(url.as_ref()),
            None => None,
        }
    }

    pub fn sentry_dsn() -> Option<&'static str> {
        match &CONFIG.sentry_dsn {
            Some(dsn) => Some(dsn.as_ref()),
            None => None,
        }
    }

    pub fn price_update_interval() -> Duration {
        match CONFIG.price_update_interval {
            Some(interval) => Duration::from_secs(interval),
            None => Duration::from_secs(120),
        }
    }

    pub fn opentelemetry_endpoint() -> &'static str {
        match &CONFIG.opentelemetry_endpoint {
            Some(endpoint) => endpoint.as_ref(),
            None => "127.0.0.1:6831",
        }
    }
}
