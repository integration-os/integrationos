use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;

const URL: &str =
    "http://metadata/computeMetadata/v1/instance/service-accounts/default/identity?audience=";
const HEADER_KEY: &str = "Metadata-Flavor";
const HEADER_VALUE: &str = "Google";

#[derive(Debug, Clone, Deserialize)]
pub struct GoogleTokenResponse {
    data: String,
}

#[derive(Debug, Clone, Default)]
pub struct GoogleTokenFetcher {
    client: Client,
}

impl GoogleTokenFetcher {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn get_token(&self, url: &str) -> Result<String> {
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
