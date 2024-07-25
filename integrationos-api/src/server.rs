use crate::{
    config::Config,
    logic::{connection_oauth_definition::FrontendOauthConnectionDefinition, openapi::OpenAPIData},
    metrics::Metric,
    router,
};
use anyhow::{anyhow, Context, Result};
use axum::Router;
use integrationos_cache::local::{
    connection_cache::ConnectionCacheArcStrHeaderKey,
    connection_definition_cache::ConnectionDefinitionCache,
    connection_oauth_definition_cache::ConnectionOAuthDefinitionCache,
    event_access_cache::EventAccessCache,
};
use integrationos_domain::{
    algebra::{CryptoExt, DefaultTemplate, MongoStore},
    common_model::{CommonEnum, CommonModel},
    connection_definition::{ConnectionDefinition, PublicConnectionDetails},
    connection_model_definition::ConnectionModelDefinition,
    connection_model_schema::{ConnectionModelSchema, PublicConnectionModelSchema},
    connection_oauth_definition::{ConnectionOAuthDefinition, Settings},
    cursor::Cursor,
    event_access::EventAccess,
    page::PlatformPage,
    stage::Stage,
    Connection, Event, Pipeline, PlatformData, Store, Transaction,
};
use integrationos_unified::unified::{UnifiedCacheTTLs, UnifiedDestination};
use mongodb::{options::UpdateOptions, Client, Database};
use segment::{AutoBatcher, Batcher, HttpClient};
use std::{sync::Arc, time::Duration};
use tokio::{net::TcpListener, sync::mpsc::Sender, time::timeout, try_join};
use tracing::{error, info, trace, warn};

#[derive(Clone)]
pub struct AppStores {
    pub db: Database,
    pub model_config: MongoStore<ConnectionModelDefinition>,
    pub oauth_config: MongoStore<ConnectionOAuthDefinition>,
    pub frontend_oauth_config: MongoStore<FrontendOauthConnectionDefinition>,
    pub model_schema: MongoStore<ConnectionModelSchema>,
    pub public_model_schema: MongoStore<PublicConnectionModelSchema>,
    pub common_model: MongoStore<CommonModel>,
    pub common_enum: MongoStore<CommonEnum>,
    pub connection: MongoStore<Connection>,
    pub public_connection_details: MongoStore<PublicConnectionDetails>,
    pub platform: MongoStore<PlatformData>,
    pub platform_page: MongoStore<PlatformPage>,
    pub settings: MongoStore<Settings>,
    pub connection_config: MongoStore<ConnectionDefinition>,
    pub pipeline: MongoStore<Pipeline>,
    pub event_access: MongoStore<EventAccess>,
    pub event: MongoStore<Event>,
    pub transactions: MongoStore<Transaction>,
    pub cursors: MongoStore<Cursor>,
    pub stages: MongoStore<Stage>,
}

#[derive(Clone)]
pub struct AppState {
    pub app_stores: AppStores,
    pub config: Config,
    pub openapi_data: OpenAPIData,
    pub http_client: reqwest::Client,
    pub event_access_cache: EventAccessCache,
    pub connections_cache: ConnectionCacheArcStrHeaderKey,
    pub connection_definitions_cache: ConnectionDefinitionCache,
    pub connection_oauth_definitions_cache: ConnectionOAuthDefinitionCache,
    pub secrets_client: Arc<dyn CryptoExt + Sync + Send>,
    pub extractor_caller: UnifiedDestination,
    pub event_tx: Sender<Event>,
    pub metric_tx: Sender<Metric>,
    pub template: DefaultTemplate,
}

#[derive(Clone)]
pub struct Server {
    state: Arc<AppState>,
}

impl Server {
    pub async fn init(
        config: Config,
        secrets_client: Arc<dyn CryptoExt + Sync + Send + 'static>,
    ) -> Result<Self> {
        let client = Client::with_uri_str(&config.db_config.control_db_url).await?;
        let db = client.database(&config.db_config.control_db_name);

        let http_client = reqwest::ClientBuilder::new()
            .timeout(Duration::from_secs(config.http_client_timeout_secs))
            .build()?;
        let model_config = MongoStore::new(&db, &Store::ConnectionModelDefinitions).await?;
        let oauth_config = MongoStore::new(&db, &Store::ConnectionOAuthDefinitions).await?;
        let frontend_oauth_config =
            MongoStore::new(&db, &Store::ConnectionOAuthDefinitions).await?;
        let model_schema = MongoStore::new(&db, &Store::ConnectionModelSchemas).await?;
        let public_model_schema =
            MongoStore::new(&db, &Store::PublicConnectionModelSchemas).await?;
        let common_model = MongoStore::new(&db, &Store::CommonModels).await?;
        let common_enum = MongoStore::new(&db, &Store::CommonEnums).await?;
        let connection = MongoStore::new(&db, &Store::Connections).await?;
        let platform = MongoStore::new(&db, &Store::Platforms).await?;
        let platform_page = MongoStore::new(&db, &Store::PlatformPages).await?;
        let public_connection_details =
            MongoStore::new(&db, &Store::PublicConnectionDetails).await?;
        let settings = MongoStore::new(&db, &Store::Settings).await?;
        let connection_config = MongoStore::new(&db, &Store::ConnectionDefinitions).await?;
        let pipeline = MongoStore::new(&db, &Store::Pipelines).await?;
        let event_access = MongoStore::new(&db, &Store::EventAccess).await?;
        let event = MongoStore::new(&db, &Store::Events).await?;
        let transactions = MongoStore::new(&db, &Store::Transactions).await?;
        let cursors = MongoStore::new(&db, &Store::Cursors).await?;
        let stages = MongoStore::new(&db, &Store::Stages).await?;

        let extractor_caller = UnifiedDestination::new(
            config.db_config.clone(),
            config.cache_size,
            secrets_client.clone(),
            UnifiedCacheTTLs {
                connection_cache_ttl_secs: config.connection_cache_ttl_secs,
                connection_model_schema_cache_ttl_secs: config
                    .connection_model_schema_cache_ttl_secs,
                connection_model_definition_cache_ttl_secs: config
                    .connection_model_definition_cache_ttl_secs,
                secret_cache_ttl_secs: config.secret_cache_ttl_secs,
            },
        )
        .await
        .with_context(|| "Could not initialize extractor caller")?;

        let app_stores = AppStores {
            db: db.clone(),
            model_config,
            oauth_config,
            platform_page,
            frontend_oauth_config,
            model_schema,
            public_model_schema,
            platform,
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

        let event_access_cache =
            EventAccessCache::new(config.cache_size, config.access_key_cache_ttl_secs);
        let connections_cache = ConnectionCacheArcStrHeaderKey::create(
            config.cache_size,
            config.connection_cache_ttl_secs,
        );
        let connection_definitions_cache = ConnectionDefinitionCache::new(
            config.cache_size,
            config.connection_definition_cache_ttl_secs,
        );
        let connection_oauth_definitions_cache = ConnectionOAuthDefinitionCache::new(
            config.cache_size,
            config.connection_oauth_definition_cache_ttl_secs,
        );
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
        let template = DefaultTemplate::default();
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
                event_access_cache,
                http_client,
                connections_cache,
                connection_definitions_cache,
                connection_oauth_definitions_cache,
                openapi_data,
                secrets_client,
                extractor_caller,
                event_tx,
                metric_tx,
                template,
            }),
        })
    }

    pub async fn run(&self) -> Result<()> {
        let app = router::get_router(&self.state).await;

        let app: Router<()> = app.with_state(self.state.clone());

        info!("Api server listening on {}", self.state.config.address);

        let tcp_listener = TcpListener::bind(&self.state.config.address).await?;

        axum::serve(tcp_listener, app.into_make_service())
            .await
            .map_err(|e| anyhow!("Server error: {}", e))
    }
}
