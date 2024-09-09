use super::MockSecretsClient;
use envconfig::Envconfig;
use http::StatusCode;
use integrationos_api::config::ConnectionsConfig as ApiConfig;
use integrationos_domain::{event_response::EventResponse, event_with_context::EventWithContext};
use integrationos_event::{
    config::EventCoreConfig, dispatcher::Dispatcher, event_handler::EventHandler,
    mongo_context_store::MongoContextStore, mongo_control_data_store::MongoControlDataStore,
};
use integrationos_gateway::config::Config as GatewayConfig;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{
    mpsc::{self, Receiver},
    Mutex,
};

#[allow(dead_code)]
#[derive(Clone)]
pub struct TestCore {
    pub config: EventCoreConfig,
    pub rx: Arc<Mutex<Receiver<()>>>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ApiResponse {
    pub code: StatusCode,
    pub data: EventResponse,
}

impl TestCore {
    #[allow(dead_code)]
    pub async fn new(
        api_config: &ApiConfig,
        gateway_config: &GatewayConfig,
        secrets_client: Arc<MockSecretsClient>,
    ) -> Self {
        let mut config = EventCoreConfig::init_from_hashmap(&HashMap::from([])).unwrap();

        config.db_config = api_config.db_config.clone();
        config.cache = gateway_config.redis.clone();

        let control_store = Arc::new(
            MongoControlDataStore::new(&config, secrets_client)
                .await
                .unwrap(),
        );

        let context_store = Arc::new(MongoContextStore::new(&config).await.unwrap());

        let dispatcher = Dispatcher {
            context_store: context_store.clone(),
            event_store: control_store.clone(),
            control_data_store: control_store.clone(),
        };

        let event_handler = EventHandler::new(config.cache.clone(), control_store, context_store)
            .await
            .unwrap();

        let (tx, rx) = mpsc::channel(100);

        tokio::task::spawn(async move {
            loop {
                let EventWithContext { context, .. } = event_handler.pop_event().await.unwrap();
                dispatcher.process_context(context).await.unwrap();
                tx.send(()).await.unwrap();
            }
        });

        Self {
            config,
            rx: Arc::new(Mutex::new(rx)),
        }
    }

    #[allow(dead_code)]
    pub async fn event_completed(&self) {
        self.rx.lock().await.recv().await.unwrap()
    }
}
