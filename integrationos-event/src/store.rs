use anyhow::Result;
use async_trait::async_trait;
use integrationos_domain::{
    algebra::PipelineExt,
    id::Id,
    {duplicates::Duplicates, extractor::HttpExtractor, Connection, Event, Pipeline},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[async_trait]
pub trait ContextStore {
    async fn get<T: PipelineExt + Clone + for<'a> Deserialize<'a> + Unpin>(
        &self,
        context_key: &Id,
    ) -> Result<T>;
    async fn set<T: PipelineExt + Clone + Serialize>(&self, context: T) -> Result<()>;
}

#[async_trait]
pub trait ControlDataStore {
    async fn fetch_connection(&self, event: &Event) -> Result<Connection>;
    async fn verify_event(&self, event: &Event) -> Result<bool>;
    async fn get_pipelines(&self, event: &Event) -> Result<Vec<Pipeline>>;
    async fn get_pipeline(&self, pipeline_key: &str) -> Result<Pipeline>;
    async fn get_extractor(&self, extractor_key: &str, pipeline_key: &str)
        -> Result<HttpExtractor>;
    async fn execute_extractor(&self, extractor: &HttpExtractor) -> Result<Value>;
    async fn send_to_destination(
        &self,
        event: &Event,
        pipeline: &Pipeline,
        context: Option<Value>,
    ) -> Result<String>;
}

#[async_trait]
pub trait EventStore {
    async fn get(&self, event_key: &Id) -> Result<Event>;
    async fn set(&self, event: Event) -> Result<()>;
    async fn get_duplicates(&self, event: &Event) -> Result<Duplicates>;
}
