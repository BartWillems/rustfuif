pub mod routes;

use crate::cache::Cache;
use crate::errors::ServiceError;

use regex::Regex;

const BASE_URI: &str = "https://duckduckgo.com";

#[derive(Debug)]
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
    #[tracing::instrument]
    async fn acquire_token(&mut self, query: &str) -> Result<&Self, ServiceError> {
        let resp = self
            .reqwest
            .get(BASE_URI)
            .query(&[("q", query)])
            .send()
            .await?
            .text()
            .await?;

        match Client::find_token(&resp) {
            Some(token) => self.token = Some(token),
            None => {
                error!("token not found in ddg request");
                return Err(ServiceError::InternalServerError);
            }
        }

        Ok(&*self)
    }

    /// look through a duckduckgo response and return the api token if it's present
    fn find_token(haystack: &str) -> Option<String> {
        lazy_static! {
            static ref TOKEN_PATTERN: Regex =
                Regex::new(r"vqd=([\d-]+)").expect("invalid ddg token regex");
        }

        TOKEN_PATTERN
            .captures(haystack)
            .and_then(|capture| capture.get(0))
            .and_then(|token| token.as_str().split('=').last())
            .map(|token| token.to_string())
    }

    #[tracing::instrument]
    pub async fn search_images(query: &str) -> Result<ImageResponse, ServiceError> {
        if let Some(res) = Cache::get(query).await {
            return Ok(res);
        }

        let mut client = Client::new();
        client.acquire_token(query).await?;

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

        Cache::set(&res, res.query.clone()).await;

        Ok(res)
    }
}

#[derive(Serialize, Deserialize)]
pub struct ImageResponse {
    query: String,
    results: Vec<Image>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_token() {
        let token = Client::find_token("nrj('/d.js?q=test&t=D&l=us-en&s=0&dl=en&ct=BE&ss_mkt=us&vqd=3-322225378556065850860803507288131703155-133178935652763664263271092398831973244&p_ent=&ex=-1&sp=0');");
        assert!(token.is_some());
        assert_eq!(
            token.unwrap(),
            String::from(
                "3-322225378556065850860803507288131703155-133178935652763664263271092398831973244"
            )
        );
    }
}
