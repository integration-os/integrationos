use super::{create, delete, read, update, CrudHook, CrudRequest};
use crate::server::{AppState, AppStores};
use axum::{routing::post, Router};
use bson::doc;
use integrationos_domain::{
    common::{
        configuration::pipeline::PipelineConfig, destination::Destination,
        event_access::EventAccess, middleware::Middleware, mongo::MongoDbStore,
        record_metadata::RecordMetadata, signature::Signature, source::Source, Pipeline,
    },
    id::{prefix::IdPrefix, Id},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub fn get_router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/:id",
            post(update::<CreatePipelineRequest, Pipeline>)
                .delete(delete::<CreatePipelineRequest, Pipeline>),
        )
        .route(
            "/",
            post(create::<CreatePipelineRequest, Pipeline>)
                .get(read::<CreatePipelineRequest, Pipeline>),
        )
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
pub struct CreatePipelineRequest {
    pub name: String,
    pub key: String,
    pub source: Source,
    pub destination: Destination,
    pub middleware: Vec<Middleware>,
    pub signature: Signature,
    pub config: PipelineConfig,
}

impl CrudHook<Pipeline> for CreatePipelineRequest {}

impl CrudRequest for CreatePipelineRequest {
    type Output = Pipeline;
    type Error = ();

    fn into_public(self) -> anyhow::Result<Self::Output, Self::Error> {
        unimplemented!()
    }

    fn into_with_event_access(self, event_access: Arc<EventAccess>) -> Self::Output {
        Self::Output {
            id: Id::now(IdPrefix::Pipeline).to_string(),
            environment: event_access.environment,
            name: self.name,
            key: self.key,
            source: self.source,
            destination: self.destination,
            middleware: self.middleware,
            ownership: event_access.ownership.clone(),
            signature: self.signature,
            config: Some(self.config),
            record_metadata: RecordMetadata::default(),
        }
    }

    fn update(self, record: &mut Self::Output) {
        let CreatePipelineRequest {
            name,
            key,
            source,
            destination,
            middleware,
            signature,
            config,
        } = self;

        record.name = name;
        record.key = key;
        record.source = source;
        record.destination = destination;
        record.middleware = middleware;
        record.signature = signature;
        record.config = Some(config);
        record.record_metadata.mark_updated(&record.ownership.id);
    }

    fn get_store(stores: AppStores) -> MongoDbStore<Self::Output> {
        stores.pipeline
    }
}
