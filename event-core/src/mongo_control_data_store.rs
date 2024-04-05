use crate::{
    config::EventCoreConfig,
    store::{ControlDataStore, EventStore},
};
use anyhow::{bail, Context as AnyhowContext, Result};
use async_trait::async_trait;
use bson::{doc, SerializerOptions};
use futures::future::join_all;
use google_token_fetcher::GoogleTokenFetcher;
use handlebars::Handlebars;
use http::header::AUTHORIZATION;
use integrationos_domain::{
    algebra::{adapter::StoreAdapter, crypto::Crypto},
    common::{
        duplicates::Duplicates,
        encrypted_access_key::EncryptedAccessKey,
        event_access::EventAccess,
        extractor::HttpExtractor,
        middleware::Middleware,
        mongo::{MongoDbStore, MongoDbStoreConfig},
        Connection, Event, Pipeline, Store,
    },
    id::Id,
    service::unified_destination::UnifiedDestination,
};
use moka::future::Cache;
use mongodb::{options::ClientOptions, Client};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tracing::{error, warn};

#[derive(Clone)]
pub struct MongoControlDataStore {
    pub connections_store: MongoDbStore<Connection>,
    pub event_store: MongoDbStore<Event>,
    pub event_access_store: MongoDbStore<EventAccess>,
    pub pipelines_store: MongoDbStore<Pipeline>,
    pub connections_cache: Cache<String, Connection>,
    pub event_cache: Cache<Id, Event>,
    pub event_access_cache: Cache<String, EventAccess>,
    pub pipelines_cache: Cache<String, Vec<Pipeline>>,
    pub pipeline_cache: Cache<String, Pipeline>,
    pub token_fetcher: Option<GoogleTokenFetcher>,
    pub http_client: reqwest::Client,
    destination_caller: UnifiedDestination,
}

impl MongoControlDataStore {
    pub async fn new(
        config: &EventCoreConfig,
        secrets_client: Arc<dyn Crypto + Sync + Send>,
    ) -> Result<Self> {
        let mut client_options = ClientOptions::parse(&config.db.control_db_url)
            .await
            .with_context(|| "Could not parse control mongodb url")?;

        client_options.max_pool_size = Some(config.db_connection_count as u32);
        let client = Client::with_options(client_options)
            .with_context(|| "Failed to create control MongoDB client with options")?;

        let db = client.database(&config.db.control_db_name);

        let connections_store =
            MongoDbStore::new(MongoDbStoreConfig::new(db.clone(), Store::Connections)).await?;
        let event_access_store =
            MongoDbStore::new(MongoDbStoreConfig::new(db.clone(), Store::EventAccess)).await?;
        let pipelines_store =
            MongoDbStore::new(MongoDbStoreConfig::new(db, Store::Pipelines)).await?;

        let mut event_client_options = ClientOptions::parse(&config.db.event_db_url)
            .await
            .with_context(|| "Could not parse events mongodb url")?;

        event_client_options.max_pool_size = Some(config.db_connection_count as u32);
        let client = Client::with_options(event_client_options)
            .with_context(|| "Failed to create events MongoDB client with options")?;

        let event_db = client.database(&config.db.event_db_name);
        let event_store = MongoDbStore::new(MongoDbStoreConfig::new(event_db, Store::Events))
            .await
            .with_context(|| {
                format!(
                    "Could not connect to event db at {}",
                    config.db.event_db_name
                )
            })?;

        Ok(Self {
            connections_store,
            event_store,
            event_access_store,
            pipelines_store,
            connections_cache: Cache::builder()
                .max_capacity(config.cache_size)
                .time_to_live(Duration::from_secs(config.cache_ttl_secs))
                .build(),
            event_cache: Cache::new(config.cache_size),
            event_access_cache: Cache::builder()
                .max_capacity(config.cache_size)
                .time_to_live(Duration::from_secs(config.cache_ttl_secs))
                .build(),
            pipelines_cache: Cache::builder()
                .max_capacity(config.cache_size)
                .time_to_live(Duration::from_secs(config.cache_ttl_secs))
                .build(),
            pipeline_cache: Cache::builder()
                .max_capacity(config.cache_size)
                .time_to_live(Duration::from_secs(config.cache_ttl_secs))
                .build(),
            token_fetcher: if config.fetch_google_auth_token {
                Some(GoogleTokenFetcher::new())
            } else {
                None
            },
            http_client: reqwest::Client::new(),
            destination_caller: UnifiedDestination::new(
                config.db.clone(),
                config.cache_size,
                secrets_client,
            )
            .await?,
        })
    }

    async fn fetch_google_auth_token(&self, url: &str) -> Option<String> {
        let token_fetcher = &(self.token_fetcher.clone()?);
        match token_fetcher.get_token(url).await {
            Ok(header) => Some(header),
            Err(_) => None,
        }
    }

    #[tracing::instrument(skip(self, event), fields(event.key = %event.key))]
    pub async fn fetch_event_access(&self, event: &Event) -> Result<Option<EventAccess>> {
        if let Some(event_access) = self.event_access_cache.get(&event.access_key).await {
            return Ok(Some(event_access));
        }

        let filter = doc! {
            "accessKey": &event.access_key,
            "deleted": false,
        };
        let Some(event_access) = self.event_access_store.get_one(filter).await.map_err(|e| {
            error!("Could not query mongodb for event: {e}");
            e
        })?
        else {
            warn!("Could not find event-access record");
            return Ok(None);
        };
        self.event_access_cache
            .insert(event.access_key.clone(), event_access.clone())
            .await;
        Ok(Some(event_access))
    }

    #[tracing::instrument(skip(self, event), fields(event.key = %event.key))]
    pub async fn fetch_pipelines(&self, event: &Event) -> Result<Vec<Pipeline>> {
        if let Some(pipelines) = self.pipelines_cache.get(&event.access_key).await {
            return Ok(pipelines);
        }
        let filter = doc! {
            "source.events": &event.name,
            "source.type": &event.r#type,
            "source.group": &event.group,
            "active": true,
            "deleted": false,
        };

        let pipelines = self
            .pipelines_store
            .get_many(Some(filter), None, None, None, None)
            .await
            .with_context(|| "Could not query mongodb for event")?;

        let futs = pipelines
            .iter()
            .map(|p| self.pipeline_cache.insert(p.key.clone(), p.clone()));
        join_all(futs).await;
        self.pipelines_cache
            .insert(event.access_key.clone(), pipelines.clone())
            .await;

        Ok(pipelines)
    }

    #[tracing::instrument(skip(self))]
    pub async fn fetch_pipeline(&self, pipeline_id: &str) -> Result<Pipeline> {
        if let Some(pipeline) = self.pipeline_cache.get(pipeline_id).await {
            return Ok(pipeline);
        }
        let Some(pipeline) = self
            .pipelines_store
            .get_one(doc! { "key": pipeline_id })
            .await
            .map_err(|e| {
                error!("Could not query mongodb for event: {e}");
                e
            })?
        else {
            bail!("Pipeline does not exist");
        };

        self.pipeline_cache
            .insert(pipeline_id.to_string(), pipeline.clone())
            .await;
        Ok(pipeline)
    }
}

#[async_trait]
impl ControlDataStore for MongoControlDataStore {
    #[tracing::instrument(skip(self, event), fields(event.key = %event.key))]
    async fn fetch_connection(&self, event: &Event) -> Result<Connection> {
        if let Some(connection) = self.connections_cache.get(&event.access_key).await {
            return Ok(connection);
        }

        let access_key = EncryptedAccessKey::parse(&event.access_key)
            .with_context(|| "Event has invalid access key")?;
        let filter = doc! {
            format!("accessKey.{}", access_key.prefix.environment): &event.access_key,
            "deleted": false,
        };
        let Some(connection) = self
            .connections_store
            .get_one(filter)
            .await
            .with_context(|| "Could not query mongodb for connection")?
        else {
            bail!("Could not find connection for event {}", event.id);
        };

        self.connections_cache
            .insert(event.access_key.clone(), connection.clone())
            .await;

        Ok(connection)
    }

    #[tracing::instrument(skip(self, event), fields(event.key = %event.key))]
    async fn verify_event(&self, event: &Event) -> Result<bool> {
        Ok(self.fetch_event_access(event).await?.is_some())
    }

    #[tracing::instrument(skip(self, event), fields(event.key = %event.key))]
    async fn get_pipelines(&self, event: &Event) -> Result<Vec<Pipeline>> {
        let pipelines = self.fetch_pipelines(event).await?;
        let mut futs = Vec::with_capacity(pipelines.len());
        for p in &pipelines {
            futs.push(self.pipeline_cache.insert(p.key.clone(), p.clone()));
        }
        join_all(futs).await;
        Ok(pipelines)
    }

    #[tracing::instrument(skip(self))]
    async fn get_pipeline(&self, pipeline_key: &str) -> Result<Pipeline> {
        match self.pipeline_cache.get(pipeline_key).await {
            Some(pipeline) => Ok(pipeline),
            None => self.fetch_pipeline(pipeline_key).await,
        }
    }

    #[tracing::instrument(skip(self))]
    async fn get_extractor(
        &self,
        extractor_key: &str,
        pipeline_key: &str,
    ) -> Result<HttpExtractor> {
        let pipeline = match self.pipeline_cache.get(pipeline_key).await {
            Some(pipeline) => pipeline,
            None => self.fetch_pipeline(pipeline_key).await?,
        };

        for e in pipeline.middleware {
            if let Middleware::HttpExtractor(e) = e {
                if e.key == extractor_key {
                    return Ok(e);
                }
            }
        }
        bail!("No extractor found")
    }

    #[tracing::instrument(skip(self))]
    async fn execute_extractor(&self, e: &HttpExtractor) -> Result<Value> {
        let auth_token = self.fetch_google_auth_token(&e.url).await;

        // Create a handlebars registry
        let handlebars = Handlebars::new();

        let context = e.context.clone();

        // Convert context to a HashMap
        let context_map: HashMap<String, Value> =
            serde_json::from_value(context.unwrap_or(serde_json::json!({})))?;

        // Process headers
        let headers_str = handlebars.render_template(&e.headers, &context_map)?;
        let headers: HashMap<String, String> = serde_json::from_str(&headers_str)?;

        // Process data (body)
        let data_str = handlebars.render_template(&e.data, &context_map)?;

        // Create a HeaderMap from processed headers
        let mut header_map = HeaderMap::new();
        for (key, value) in headers.iter() {
            let header_name = HeaderName::from_lowercase(key.as_bytes())?;
            let header_value = HeaderValue::from_str(value)?;
            header_map.insert(header_name, header_value);
        }

        if let Some(ref token) = auth_token {
            header_map.insert(AUTHORIZATION, token.try_into()?);
        }

        let response = self
            .http_client
            .request(e.method.clone(), &e.url)
            .headers(header_map)
            .body(data_str)
            .send()
            .await?;

        if response.status().is_success() {
            let mut headers = HashMap::new();
            for (k, v) in response.headers() {
                let k = k.to_string();
                let v = String::from_utf8(v.as_bytes().to_vec())?;
                headers.insert(k, v);
            }
            let response_body = response.json::<Value>().await?;
            return Ok(json!({
                "headers": headers,
                "data": response_body
            }));
        } else {
            bail!(format!(
                "Extractor failed: {} - {}",
                response.status(),
                response.text().await?
            ));
        }
    }

    #[tracing::instrument(skip(self, pipeline), fields(pipeline.id = %pipeline.id))]
    async fn send_to_destination(
        &self,
        event: &Event,
        pipeline: &Pipeline,
        context: Option<Value>,
    ) -> Result<String> {
        let response = self
            .destination_caller
            .send_to_destination(
                None,
                &pipeline.destination,
                event.headers.clone(),
                HashMap::new(),
                context.and_then(|c| serde_json::to_vec(&c).ok()),
            )
            .await
            .with_context(|| "Error sending event to destination")?;

        let response_string = response.text().await?;

        Ok(response_string)
    }
}

#[async_trait]
impl EventStore for MongoControlDataStore {
    #[tracing::instrument(skip(self), fields(event_key = %event_key))]
    async fn get(&self, event_key: &Id) -> Result<Event> {
        if let Some(event) = self.event_cache.get(event_key).await {
            return Ok(event);
        }

        let Some(event) = self
            .event_store
            .get_one_by_id(event_key.to_string().as_str())
            .await
            .map_err(|e| {
                error!("Could not query mongodb for event: {e}");
                e
            })?
        else {
            bail!("Could not find event");
        };
        self.event_cache.insert(*event_key, event.clone()).await;
        Ok(event)
    }

    #[tracing::instrument(skip(self, event), fields(event.key = %event.key))]
    async fn set(&self, event: Event) -> Result<()> {
        let options = SerializerOptions::builder().human_readable(false).build();
        self.event_store
            .update_one(
                &event.id.to_string(),
                doc! { "$set": {
                    "duplicates": bson::to_bson_with_options(&event.duplicates, options.clone())?,
                    "createdAt": bson::to_bson_with_options(&event.record_metadata.created_at, options.clone())?,
                    "state": bson::to_bson_with_options(&event.state, options)?
                } },
            )
            .await?;
        self.event_cache.insert(event.id, event.clone()).await;
        Ok(())
    }

    #[tracing::instrument(skip(self, event), fields(event.key = %event.key))]
    async fn get_duplicates(&self, event: &Event) -> Result<Duplicates> {
        let query = doc! {
            "$or": [
                {
                    "hashes.hash": {
                        "$eq": &event.hashes[0].hash,
                    }
                },
                {
                    "hashes.hash": {
                        "$eq": &event.hashes[1].hash,
                    }
                },
                {
                    "hashes.hash": {
                        "$eq": &event.hashes[2].hash,
                    }
                },
            ],
            "_id": {
                "$ne": event.id.to_string()
            }
        };
        let duplicate_count = self.event_store.count(query, Some(1)).await?;

        Ok(Duplicates {
            possible_collision: duplicate_count == 1,
        })
    }
}
