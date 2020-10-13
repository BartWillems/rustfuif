pub mod routes;

use crate::cache;
use crate::errors::ServiceError;

use regex::Regex;
use reqwest;

const BASE_URI: &str = "https://duckduckgo.com";

#[derive(Clone)]
pub struct Client {
    token: Option<String>,
    reqwest: reqwest::Client,
}

impl Client {
    fn new() -> Self {
        Client {
            token: None,
            reqwest: reqwest::Client::new(),
        }
    }
    /// fetch and set the duckduckgo request token
    /// This token is only valid for a specific request for a (currently unkown) amount of time
    async fn acquire_token(&mut self, query: &str) -> Result<&Client, ServiceError> {
        let resp = self
            .reqwest
            .get(BASE_URI)
            .query(&[("q", query)])
            .send()
            .await?
            .text()
            .await?;

        lazy_static! {
            static ref TOKEN_PATTERN: Regex =
                Regex::new(r"vqd=([\d-]+)").expect("invalid ddg token regex");
        }

        let capture = TOKEN_PATTERN
            .captures(&resp)
            .and_then(|capture| capture.get(0))
            .and_then(|token| token.as_str().split('=').last());

        match capture {
            Some(token) => self.token = Some(token.into()),
            None => {
                error!("token not found in ddg request");
                return Err(ServiceError::InternalServerError);
            }
        }

        Ok(self)
    }

    pub async fn search_images(query: &str) -> Result<ImageResponse, ServiceError> {
        if let Some(res) = cache::find(query).unwrap_or(None) {
            return Ok(res);
        }
        let client: Client = Client::new().acquire_token(query).await?.to_owned();

        let res = client
            .reqwest
            .get(format!("{}/i.js", BASE_URI).as_str())
            .query(&[
                ("l", "us-en"),
                ("o", "json"),
                (
                    "vqd",
                    client
                        .token
                        .expect("By this point the DDG token should exist")
                        .as_str(),
                ),
                ("q", query),
            ])
            .send()
            .await?
            .json::<ImageResponse>()
            .await?;

        if let Err(e) = cache::set(&res, res.query.clone()) {
            error!("unable to cache the DDG image query: {}", e);
        }

        Ok(res)
    }
}

#[derive(Serialize, Deserialize)]
pub struct ImageResponse {
    query: String,
    results: Vec<Image>,
}

impl crate::cache::Cache for ImageResponse {
    fn cache_key<T: std::fmt::Display>(id: T) -> String {
        format!("image_response.{}", id)
    }
}

#[derive(Serialize, Deserialize)]
pub struct Image {
    width: i32,
    height: i32,
    url: String,
    source: String,
    title: String,
    image: String,
}

impl From<reqwest::Error> for ServiceError {
    fn from(error: reqwest::Error) -> ServiceError {
        error!("reqwest error: {}", error);
        ServiceError::InternalServerError
    }
}