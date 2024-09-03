use crate::{
    configuration::secrets::SecretsConfig,
    microservice::MicroService,
    prelude::{
        create_secret_request::CreateSecretRequest, create_secret_response::CreateSecretResponse,
        get_secret_request::GetSecretRequest, get_secret_response::GetSecretResponse, MongoStore,
    },
    IntegrationOSError, InternalError, SecretVersion, Store,
};
use bson::doc;
use mongodb::Client;
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub struct SecretsClient {
    get_url: String,
    create_url: String,
}

impl SecretsClient {
    pub fn new(config: &SecretsConfig) -> Result<Self, IntegrationOSError> {
        Ok(Self {
            get_url: format!("{}{}", config.base_url, config.get_path),
            create_url: format!("{}{}", config.base_url, config.create_path),
        })
    }

    pub async fn get_secret<T: for<'a> Deserialize<'a> + Debug>(
        &self,
        data: &GetSecretRequest,
    ) -> Result<T, IntegrationOSError> {
        let client = self.get_client().await?;

        let response = client
            .post(&self.get_url)
            .json(data)
            .send()
            .await
            .map_err(|err| {
                InternalError::io_err(&format!("Failed to send request: {err}"), None)
            })?;

        let body_str = self.handle_response(response).await?;

        let body: GetSecretResponse<Value> = serde_json::from_str(&body_str).map_err(|err| {
            InternalError::invalid_argument(&format!("Failed to deserialize response: {err}"), None)
        })?;

        let secret_str = body.secret.as_str().ok_or(InternalError::invalid_argument(
            "Failed to get secret",
            None,
        ))?;

        let secret = serde_json::from_str::<T>(secret_str).map_err(|err| {
            InternalError::invalid_argument(&format!("Failed to deserialize secret: {err }"), None)
        })?;

        Ok(secret)
    }

    pub async fn create_secret<T: Serialize>(
        &self,
        buildable_id: String,
        secret: T,
    ) -> Result<CreateSecretResponse, IntegrationOSError> {
        let secret = serde_json::to_string(&secret).unwrap();

        let client = self.get_client().await?;

        let data = &CreateSecretRequest {
            buildable_id,
            secret,
            version: SecretVersion::V2,
        };

        let response = client
            .post(&self.create_url)
            .json(data)
            .send()
            .await
            .map_err(|err| {
                InternalError::io_err(&format!("Failed to send request: {err}"), None)
            })?;

        let body_str = self.handle_response(response).await?;

        let body = serde_json::from_str::<CreateSecretResponse>(&body_str).map_err(|err| {
            InternalError::invalid_argument(&format!("Failed to deserialize response: {err}"), None)
        })?;

        Ok(body)
    }

    async fn handle_response(
        &self,
        response: reqwest::Response,
    ) -> Result<String, IntegrationOSError> {
        let status = response.status();
        let content = response.text().await.map_err(|err| {
            InternalError::io_err(&format!("Failed to read response: {err}"), None)
        })?;

        if status.is_client_error() || status.is_server_error() {
            Err(InternalError::invalid_argument(
                &format!("Response Status: {}", status.as_u16()),
                None,
            ))
        } else {
            Ok(content)
        }
    }

    pub async fn get_base_url(db_uri: &str, db_name: &str) -> Result<String, IntegrationOSError> {
        let filter = doc! {
            "name": "secrets-service",
        };

        let database = Client::with_uri_str(db_uri).await?.database(db_name);
        let store = MongoStore::<MicroService>::new(&database, &Store::MicroServices).await?;

        let microservice = store
            .get_one(filter)
            .await?
            .ok_or_else(|| InternalError::key_not_found("Microservice not found", None))?;

        Ok(microservice.microservice.url)
    }

    async fn get_client(&self) -> Result<reqwest::Client, IntegrationOSError> {
        ReqwestClient::builder().build().map_err(|err| {
            InternalError::connection_error(&format!("Failed to create client: {err}"), None)
        })
    }
}
