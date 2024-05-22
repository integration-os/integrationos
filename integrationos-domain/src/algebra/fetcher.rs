use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

const URL: &str =
    "http://metadata/computeMetadata/v1/instance/service-accounts/default/identity?audience=";
const HEADER_KEY: &str = "Metadata-Flavor";
const HEADER_VALUE: &str = "Google";

#[async_trait]
pub trait FecherExt {
    async fn get_token(&self, url: &str) -> Result<String>;
}

#[derive(Debug, Clone, Deserialize)]
pub struct GoogleTokenResponse {
    data: String,
}

#[derive(Debug, Clone, Default)]
pub struct GoogleTokenFetcher {
    client: Client,
}

#[async_trait]
impl FecherExt for GoogleTokenFetcher {
    async fn get_token(&self, url: &str) -> Result<String> {
        let res = self
            .client
            .get(format!("{URL}{url}"))
            .header(HEADER_KEY, HEADER_VALUE)
            .send()
            .await?
            .json::<GoogleTokenResponse>()
            .await?;

        Ok(format!("Bearer {}", res.data))
    }
}

impl GoogleTokenFetcher {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}
