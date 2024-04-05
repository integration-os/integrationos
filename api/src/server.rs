use crate::{
    config::Config,
    endpoints::{
        connection_oauth_definition::FrontendOauthConnectionDefinition, openapi::OpenAPIData,
        GetCache,
    },
    metrics::Metric,
    routes,
};
use anyhow::{anyhow, Context, Result};
use axum::Router;
use http::HeaderValue;
use integrationos_domain::{
    algebra::crypto::Crypto,
    common::{
        common_model::CommonModel,
        connection_definition::ConnectionDefinition,
        connection_model_definition::ConnectionModelDefinition,
        connection_model_schema::{ConnectionModelSchema, PublicConnectionModelSchema},
        connection_oauth_definition::{ConnectionOAuthDefinition, Settings},
        cursor::Cursor,
        event_access::EventAccess,
        mongo::MongoDbStore,
        stage::Stage,
        Connection, Event, Pipeline, Store, Transaction,
    },
    common_model::CommonEnum,
    connection_definition::PublicConnectionDetails,
    service::unified_destination::UnifiedDestination,
};
use moka::future::Cache;
use mongodb::{options::UpdateOptions, Client, Database};
use segment::{AutoBatcher, Batcher, HttpClient};
use std::{sync::Arc, time::Duration};
use tokio::{sync::mpsc::Sender, time::timeout, try_join};
use tracing::{error, info, trace, warn};

#[derive(Clone)]
pub struct AppStores {
    pub db: Database,
    pub model_config: MongoDbStore<ConnectionModelDefinition>,
    pub oauth_config: MongoDbStore<ConnectionOAuthDefinition>,
    pub frontend_oauth_config: MongoDbStore<FrontendOauthConnectionDefinition>,
    pub model_schema: MongoDbStore<ConnectionModelSchema>,
    pub public_model_schema: MongoDbStore<PublicConnectionModelSchema>,
    pub common_model: MongoDbStore<CommonModel>,
    pub common_enum: MongoDbStore<CommonEnum>,
    pub connection: MongoDbStore<Connection>,
    pub public_connection_details: MongoDbStore<PublicConnectionDetails>,
    pub settings: MongoDbStore<Settings>,
    pub connection_config: MongoDbStore<ConnectionDefinition>,
    pub pipeline: MongoDbStore<Pipeline>,
    pub event_access: MongoDbStore<EventAccess>,
    pub event: MongoDbStore<Event>,
    pub transactions: MongoDbStore<Transaction>,
    pub cursors: MongoDbStore<Cursor>,
    pub stages: MongoDbStore<Stage>,
}

#[derive(Clone)]
pub struct AppState {
    pub app_stores: AppStores,
    pub config: Config,
    pub cache: Cache<HeaderValue, Arc<EventAccess>>,
    pub openapi_data: OpenAPIData,
    pub http_client: reqwest::Client,
    pub connections_cache: Cache<(Arc<str>, HeaderValue), Arc<Connection>>,
    pub connection_definitions_cache: GetCache<ConnectionDefinition>,
    pub connection_oauth_definitions_cache: GetCache<FrontendOauthConnectionDefinition>,
    pub secrets_client: Arc<dyn Crypto + Sync + Send>,
    pub extractor_caller: UnifiedDestination,
    pub event_tx: Sender<Event>,
    pub metric_tx: Sender<Metric>,
}

#[derive(Clone)]
pub struct Server {
    state: Arc<AppState>,
}

impl Server {
    pub async fn init(
        config: Config,
        secrets_client: Arc<dyn Crypto + Sync + Send + 'static>,
    ) -> Result<Self> {
        let client = Client::with_uri_str(&config.db_config.control_db_url).await?;
        let db = client.database(&config.db_config.control_db_name);

        let http_client = reqwest::ClientBuilder::new()
            .timeout(Duration::from_secs(config.http_client_timeout_secs))
            .build()?;
        let model_config =
            MongoDbStore::new_with_db(db.clone(), Store::ConnectionModelDefinitions).await?;
        let oauth_config =
            MongoDbStore::new_with_db(db.clone(), Store::ConnectionOAuthDefinitions).await?;
        let frontend_oauth_config =
            MongoDbStore::new_with_db(db.clone(), Store::ConnectionOAuthDefinitions).await?;
        let model_schema =
            MongoDbStore::new_with_db(db.clone(), Store::ConnectionModelSchemas).await?;
        let public_model_schema =
            MongoDbStore::new_with_db(db.clone(), Store::PublicConnectionModelSchemas).await?;
        let common_model = MongoDbStore::new_with_db(db.clone(), Store::CommonModels).await?;
        let common_enum = MongoDbStore::new_with_db(db.clone(), Store::CommonEnums).await?;
        let connection = MongoDbStore::new_with_db(db.clone(), Store::Connections).await?;
        let public_connection_details =
            MongoDbStore::new_with_db(db.clone(), Store::PublicConnectionDetails).await?;
        let settings = MongoDbStore::new_with_db(db.clone(), Store::Settings).await?;
        let connection_config =
            MongoDbStore::new_with_db(db.clone(), Store::ConnectionDefinitions).await?;
        let pipeline = MongoDbStore::new_with_db(db.clone(), Store::Pipelines).await?;
        let event_access = MongoDbStore::new_with_db(db.clone(), Store::EventAccess).await?;
        let event = MongoDbStore::new_with_db(db.clone(), Store::Events).await?;
        let transactions = MongoDbStore::new_with_db(db.clone(), Store::Transactions).await?;
        let cursors = MongoDbStore::new_with_db(db.clone(), Store::Cursors).await?;
        let stages = MongoDbStore::new_with_db(db.clone(), Store::Stages).await?;

        let extractor_caller = UnifiedDestination::new(
            config.db_config.clone(),
            config.cache_size,
            secrets_client.clone(),
        )
        .await
        .with_context(|| "Could not initialize extractor caller")?;

        let app_stores = AppStores {
            db: db.clone(),
            model_config,
            oauth_config,
            frontend_oauth_config,
            model_schema,
            public_model_schema,
            settings,
            common_model,
            common_enum,
            connection,
            public_connection_details,
            connection_config,
            pipeline,
            event_access,
            event,
            transactions,
            cursors,
            stages,
        };

        let cache = Cache::builder()
            .max_capacity(config.cache_size)
            .time_to_live(Duration::from_secs(config.access_key_cache_ttl_secs))
            .build();
        let connections_cache = Cache::new(config.cache_size);
        let connection_definitions_cache =
            Arc::new(Cache::builder().max_capacity(config.cache_size).build());
        let connection_oauth_definitions_cache =
            Arc::new(Cache::builder().max_capacity(config.cache_size).build());
        let openapi_data = OpenAPIData::default();
        openapi_data.spawn_openapi_generation(
            app_stores.common_model.clone(),
            app_stores.common_enum.clone(),
        );

        // Create Event buffer in separate thread and batch saves
        let events = db.collection::<Event>(&Store::Events.to_string());
        let (event_tx, mut receiver) =
            tokio::sync::mpsc::channel::<Event>(config.event_save_buffer_size);
        tokio::spawn(async move {
            let mut buffer = Vec::with_capacity(config.event_save_buffer_size);
            loop {
                let res = timeout(
                    Duration::from_secs(config.event_save_timeout_secs),
                    receiver.recv(),
                )
                .await;
                let is_timeout = if let Ok(Some(event)) = res {
                    buffer.push(event);
                    false
                } else if let Ok(None) = res {
                    break;
                } else {
                    trace!("Event receiver timed out waiting for new event");
                    true
                };
                // Save when buffer is full or timeout elapsed
                if buffer.len() == config.event_save_buffer_size
                    || (is_timeout && !buffer.is_empty())
                {
                    trace!("Saving {} events", buffer.len());
                    let to_save = std::mem::replace(
                        &mut buffer,
                        Vec::with_capacity(config.event_save_buffer_size),
                    );
                    let events = events.clone();
                    tokio::spawn(async move {
                        if let Err(e) = events.insert_many(to_save, None).await {
                            error!("Could not save buffer of events: {e}");
                        }
                    });
                }
            }
        });

        // Update metrics in separate thread
        let client = HttpClient::default();
        let batcher = Batcher::new(None);
        let mut batcher = config
            .segment_write_key
            .as_ref()
            .map(|k| AutoBatcher::new(client, batcher, k.to_string()));

        let metrics = db.collection::<Metric>(&Store::Metrics.to_string());
        let (metric_tx, mut receiver) =
            tokio::sync::mpsc::channel::<Metric>(config.metric_save_channel_size);
        let metric_system_id = config.metric_system_id.clone();
        tokio::spawn(async move {
            let options = UpdateOptions::builder().upsert(true).build();

            loop {
                let res = timeout(
                    Duration::from_secs(config.event_save_timeout_secs),
                    receiver.recv(),
                )
                .await;
                if let Ok(Some(metric)) = res {
                    let doc = metric.update_doc();
                    let client = metrics.update_one(
                        bson::doc! {
                            "clientId": &metric.ownership().client_id,
                        },
                        doc.clone(),
                        options.clone(),
                    );
                    let system = metrics.update_one(
                        bson::doc! {
                            "clientId": metric_system_id.as_str(),
                        },
                        doc,
                        options.clone(),
                    );
                    if let Err(e) = try_join!(client, system) {
                        error!("Could not upsert metric: {e}");
                    }

                    if let Some(ref mut batcher) = batcher {
                        let msg = metric.segment_track();
                        if let Err(e) = batcher.push(msg).await {
                            warn!("Tracking msg is too large: {e}");
                        }
                    }
                } else if let Ok(None) = res {
                    break;
                } else {
                    trace!("Event receiver timed out waiting for new event");
                    if let Some(ref mut batcher) = batcher {
                        if let Err(e) = batcher.flush().await {
                            warn!("Tracking flush is too large: {e}");
                        }
                    }
                }
            }
            if let Some(ref mut batcher) = batcher {
                if let Err(e) = batcher.flush().await {
                    warn!("Tracking flush is too large: {e}");
                }
            }
        });

        Ok(Self {
            state: Arc::new(AppState {
                app_stores,
                config,
                cache,
                http_client,
                connections_cache,
                connection_definitions_cache,
                connection_oauth_definitions_cache,
                openapi_data,
                secrets_client,
                extractor_caller,
                event_tx,
                metric_tx,
            }),
        })
    }

    pub async fn run(&self) -> Result<()> {
        let app = self.get_router().await;

        let app: Router<()> = app.with_state(self.state.clone());

        info!("Api server listening on {}", self.state.config.address);

        axum::Server::bind(&self.state.config.address)
            .serve(app.into_make_service())
            .await
            .map_err(|e| anyhow!("Server error: {}", e))
    }

    async fn get_router(&self) -> Router<Arc<AppState>> {
        if self.state.config.is_admin {
            routes::get_admin_router(&self.state)
        } else {
            routes::get_public_router(&self.state).await
        }
    }
}
