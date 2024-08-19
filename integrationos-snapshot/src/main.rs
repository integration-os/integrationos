mod config;

use std::{
    collections::{HashMap, HashSet},
    fmt::format,
};

use anyhow::{Context, Result};
use bson::{doc, Document};
use chrono::{Duration, Utc};
use config::SnapshotConfig;
use dotenvy::dotenv;
use envconfig::Envconfig;
use futures::TryStreamExt;
use integrationos_domain::{
    telemetry::{get_subscriber, init_subscriber},
    Event, Id, MongoStore, Store, Unit,
};
use mongodb::{Client, ClientSession};
use serde::{Deserialize, Serialize};
use tracing::info;

#[tokio::main]
async fn main() -> Result<Unit> {
    dotenv().ok();

    let suscriber = get_subscriber("snapshot".into(), "info".into(), std::io::stdout);
    init_subscriber(suscriber);

    let snapshot_config = SnapshotConfig::init_from_env().context("Could not load config")?;

    info!("Starting snapshot with config: {snapshot_config}");

    let client = Client::with_uri_str(&snapshot_config.db.control_db_url)
        .await
        .context("Could not connect to mongodb")?;
    let event_db = client.database(&snapshot_config.db.control_db_name);

    let mut session = client.start_session(None).await?;

    let event_store: MongoStore<Document> = MongoStore::new(&event_db, &Store::Events)
        .await
        .with_context(|| {
            format!(
                "Could not connect to event db at {}",
                snapshot_config.db.control_db_name
            )
        })?;

    snapshot(event_store, &mut session, &snapshot_config).await?;

    Ok(())
}

#[derive(Debug, Clone, Deserialize)]
struct CorruptedEvent {
    #[serde(rename = "_id")]
    id: Id,
}

async fn snapshot(
    event_store: MongoStore<Document>,
    session: &mut ClientSession,
    config: &SnapshotConfig,
) -> Result<Unit> {
    let filter = doc! {
        "createdAt": {
            "$lt": (Utc::now() - Duration::days(30)).timestamp_millis()
        }
    };

    let mut events = event_store
        .collection
        .find_with_session(filter, None, session)
        .await
        .context("Error fetching events")?;

    events
        .stream(session)
        .try_chunks(config.stream_chunk_size)
        .try_for_each_concurrent(config.stream_concurrency, |chunk| async move {
            let mut corrupted_events = HashSet::new();
            for event in chunk {
                // Attempt to deserialize the event as a regular Event
                let decoded_event: Result<Event> =
                    bson::from_document(event.clone()).context("Could not deserialize event");

                match decoded_event {
                    Ok(event) => {
                        tracing::info!("Event with id {} received", event.id);
                    }
                    Err(e) => {
                        // Attempt to deserialize the corrupted event
                        let corrupted_event: Result<CorruptedEvent> =
                            bson::from_document(event.clone()).context(format!(
                                "Could not deserialize corrupted event to a known type {e:?}"
                            ));

                        match corrupted_event {
                            Ok(corrupted_event) => {
                                corrupted_events.insert(corrupted_event.id);
                            }
                            Err(_) => {
                                tracing::error!(
                                    "Unknown source of corruption, please contact the platform team for assistance: {event:?}"
                                );
                            }
                        }
                    }
                }
            }

            Ok(())
        })
        .await
        .context("Error streaming events")?;

    Ok(())
}

// async fn restore(events: MongoStore<Event>, session: &mut ClientSession) -> Result<Unit> {
//     let filter = doc! {
//         "createdAt": {
//             "$lt": (Utc::now() - Duration::days(30)).timestamp_millis()
//         }
//     };

//     let mut events = events
//         .collection
//         .find_with_session(filter, None, session)
//         .await?;

//     events.stream(session).try_chunks(100)

//         .map(|chunk| {
//         //
//         //
//         //
//         todo!()
//     });

//     todo!()
// }
