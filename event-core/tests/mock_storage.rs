use anyhow::{anyhow, bail, Result};
use async_trait::async_trait;
use chrono::Utc;
use event_core::{
    dispatcher::Dispatcher,
    store::{ContextStore, ControlDataStore, EventStore},
};
use fake::{Fake, Faker};
use integrationos_domain::{
    algebra::PipelineExt,
    common::{
        duplicates::Duplicates, extractor::HttpExtractor, Connection, Event, ExtractorContext,
        Pipeline, PipelineContext, RootContext,
    },
    id::{prefix::IdPrefix, Id},
    pipeline_context::PipelineStage,
    root_context::RootStage,
};
use serde_json::Value;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

type Contexts = Arc<Mutex<HashMap<Id, Vec<Box<dyn PipelineExt>>>>>;

#[derive(Clone, Default)]
pub struct MockStorage {
    pub contexts: Contexts,
    pub pipelines: Arc<Mutex<HashMap<String, Pipeline>>>,
    pub events: Arc<Mutex<HashMap<Id, Event>>>,
    pub drop_at: Option<RootStage>,
    pub fail_at: Option<RootStage>,
    pub fail_pipeline_at: Option<PipelineStage>,
}

impl MockStorage {
    pub fn new() -> Self {
        Self {
            contexts: Arc::new(Mutex::new(HashMap::new())),
            pipelines: Arc::new(Mutex::new(HashMap::new())),
            events: Arc::new(Mutex::new(HashMap::new())),
            drop_at: None,
            fail_at: None,
            fail_pipeline_at: None,
        }
    }
}

#[async_trait]
impl ContextStore for MockStorage {
    async fn get<T: PipelineExt + Clone>(&self, context_key: &Id) -> Result<T> {
        self.contexts
            .lock()
            .unwrap()
            .get(context_key)
            .map(|c| {
                let last = c.last();
                last.expect("No context for {context_key}")
                    .downcast_ref::<T>()
                    .expect("PipelineExt could not be downcast")
                    .clone()
            })
            .ok_or(anyhow!("No context for {context_key}"))
    }

    async fn set<T: PipelineExt + Clone>(&self, context: T) -> Result<()> {
        let context = Box::new(context);
        self.contexts
            .lock()
            .unwrap()
            .entry(*context.context_key())
            .and_modify(|v| v.push(context.clone()))
            .or_insert(vec![context]);
        Ok(())
    }
}

macro_rules! fail_at {
    ($fail_at:expr, $stage:pat, $message:expr) => {
        if matches!($fail_at, $stage) {
            bail!($message)
        }
    };
}

#[async_trait]
impl ControlDataStore for MockStorage {
    async fn fetch_connection(&self, _event: &Event) -> Result<Connection> {
        unimplemented!()
    }

    async fn verify_event(&self, _event: &Event) -> Result<bool> {
        fail_at!(
            self.fail_at,
            Some(RootStage::ProcessedDuplicates),
            "Failed to fetch event"
        );
        Ok(self.drop_at != Some(RootStage::ProcessedDuplicates))
    }

    async fn get_pipelines(&self, _event: &Event) -> Result<Vec<Pipeline>> {
        fail_at!(
            self.fail_at,
            Some(RootStage::Verified),
            "Failed to get pipelines"
        );
        Ok(self.pipelines.lock().unwrap().values().cloned().collect())
    }

    async fn get_pipeline(&self, pipeline_key: &str) -> Result<Pipeline> {
        fail_at!(
            self.fail_pipeline_at,
            Some(PipelineStage::New),
            "Failed to get pipeline"
        );
        self.pipelines
            .lock()
            .unwrap()
            .get(pipeline_key)
            .ok_or(anyhow!("Could not find pipeline for key {pipeline_key}"))
            .cloned()
    }

    async fn get_extractor(
        &self,
        _extractor_key: &str,
        _pipeline_key: &str,
    ) -> Result<HttpExtractor> {
        fail_at!(
            self.fail_pipeline_at,
            Some(PipelineStage::ExecutingExtractors(..)),
            "Failed to get extractor"
        );
        unimplemented!()
    }

    async fn execute_extractor(&self, _extractor: &HttpExtractor) -> Result<Value> {
        fail_at!(
            self.fail_pipeline_at,
            Some(PipelineStage::ExecutingExtractors(..)),
            "Failed to get extractor"
        );
        unimplemented!()
    }

    async fn send_to_destination(
        &self,
        _event: &Event,
        _pipeline: &Pipeline,
        _context: Option<Value>,
    ) -> Result<String> {
        Ok("{}".to_string())
    }
}

#[async_trait]
impl EventStore for MockStorage {
    async fn get(&self, event_key: &Id) -> Result<Event> {
        self.events
            .lock()
            .unwrap()
            .get(event_key)
            .ok_or(anyhow!("Could not find event with key {event_key}"))
            .cloned()
    }

    async fn set(&self, event: Event) -> Result<()> {
        self.events.lock().unwrap().insert(event.key, event);
        Ok(())
    }

    async fn get_duplicates(&self, _event: &Event) -> Result<Duplicates> {
        Ok(Duplicates {
            possible_collision: true,
        })
    }
}

#[tokio::test]
async fn get_and_set_contexts_downcasting_works() {
    let store = MockStorage::default();
    let id = Id::new(IdPrefix::Event, Utc::now());

    let context = RootContext::new(id);
    ContextStore::set(&store, context.clone()).await.unwrap();
    assert_eq!(
        context,
        ContextStore::get(&store, context.context_key())
            .await
            .unwrap()
    );

    let id = Id::new(IdPrefix::Event, Utc::now());
    let context = PipelineContext::new(id.to_string(), &context);
    ContextStore::set(&store, context.clone()).await.unwrap();
    assert_eq!(
        context,
        ContextStore::get(&store, context.context_key())
            .await
            .unwrap()
    );

    let id = Id::new(IdPrefix::Event, Utc::now());
    let context = ExtractorContext::new(id.to_string(), &context);
    ContextStore::set(&store, context.clone()).await.unwrap();
    assert_eq!(
        context,
        ContextStore::get(&store, context.context_key())
            .await
            .unwrap()
    );
}

impl MockStorage {
    fn get_at<T: PipelineExt + Clone>(&self, index: usize) -> T {
        let c = self.contexts.lock().unwrap();
        let c = c.values().flatten().collect::<Vec<_>>();

        let last = c.get(index);
        last.expect("No context for {context_key}")
            .downcast_ref::<T>()
            .expect("PipelineExt could not be downcast")
            .clone()
    }
}

#[tokio::test]
#[ignore]
async fn run_dispatcher() {
    let mut event: Event = Faker.fake();
    event.access_key = "id_live_1_abcd".to_owned();
    let store = Arc::new(MockStorage::new());
    store.events.lock().unwrap().insert(event.id, event.clone());

    let pipeline: Pipeline = Faker.fake();
    store
        .pipelines
        .lock()
        .unwrap()
        .insert(pipeline.id.clone(), pipeline.clone());

    let dispatcher = Dispatcher {
        context_store: store.clone(),
        event_store: store.clone(),
        control_data_store: store.clone(),
    };

    let context = RootContext::new(event.id);
    let res = dispatcher.process_context(context).await;
    assert!(res.is_ok());

    let context = RootContext::new(event.id);
    let res = dispatcher.process_root_context(context).await;

    assert!(res.is_ok());

    macro_rules! root_context {
        ($stage:expr) => {{
            let mut context = RootContext::new(event.id.clone());
            context.stage = $stage;
            context
        }};
    }

    macro_rules! pipeline_context {
        ($stage:expr) => {{
            let context = RootContext::new(event.id.clone());
            let mut context = PipelineContext::new(pipeline.id.clone(), &context);
            context.stage = $stage;
            context
        }};
    }

    for i in 0..7 {
        match i {
            0 => assert_eq!(root_context!(RootStage::Verified), store.get_at(i)),
            1 => assert_eq!(
                root_context!(RootStage::ProcessedDuplicates),
                store.get_at(i)
            ),
            2 => {
                let mut map = HashMap::new();
                map.insert(pipeline.id.clone(), pipeline_context!(PipelineStage::New));
                assert_eq!(
                    root_context!(RootStage::ProcessingPipelines(map)),
                    store.get_at(i)
                );
            }
            3 => {
                let map = HashMap::new();
                assert_eq!(
                    pipeline_context!(PipelineStage::ExecutingExtractors(map)),
                    store.get_at(i)
                );
            }
            4 => assert_eq!(
                pipeline_context!(PipelineStage::ExecutedExtractors(HashMap::new())),
                store.get_at(i)
            ),
            5 => assert_eq!(
                pipeline_context!(PipelineStage::ExecutedTransformer(None)),
                store.get_at(i)
            ),
            6 => assert_eq!(
                pipeline_context!(PipelineStage::FinishedPipeline),
                store.get_at(i)
            ),
            _ => {
                panic!("We should not have this many")
            }
        }
    }
}
