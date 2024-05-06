use super::{create, delete, read, update, HookExt, RequestExt};
use crate::server::{AppState, AppStores};
use axum::{routing::post, Router};
use bson::doc;
use integrationos_domain::{
    algebra::MongoStore,
    configuration::pipeline::PipelineConfig,
    destination::Destination,
    event_access::EventAccess,
    id::{prefix::IdPrefix, Id},
    middleware::Middleware,
    record_metadata::RecordMetadata,
    signature::Signature,
    source::Source,
    Pipeline,
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

impl HookExt<Pipeline> for CreatePipelineRequest {}

impl RequestExt for CreatePipelineRequest {
    type Output = Pipeline;

    fn access(&self, event_access: Arc<EventAccess>) -> Option<Self::Output> {
        Some(Self::Output {
            id: Id::now(IdPrefix::Pipeline).to_string(),
            environment: event_access.environment,
            name: self.name.clone(),
            key: self.key.clone(),
            source: self.source.clone(),
            destination: self.destination.clone(),
            middleware: self.middleware.clone(),
            ownership: event_access.ownership.clone(),
            signature: self.signature.clone(),
            config: Some(self.config.clone()),
            record_metadata: RecordMetadata::default(),
        })
    }

    fn update(&self, mut record: Self::Output) -> Self::Output {
        let CreatePipelineRequest {
            name,
            key,
            source,
            destination,
            middleware,
            signature,
            config,
        } = self;

        record.name = name.into();
        record.key = key.into();
        record.source = source.clone();
        record.destination = destination.clone();
        record.middleware = middleware.clone();
        record.signature = signature.clone();
        record.config = Some(config.clone());
        record.record_metadata.mark_updated(&record.ownership.id);

        record
    }

    fn get_store(stores: AppStores) -> MongoStore<Self::Output> {
        stores.pipeline
    }
}
