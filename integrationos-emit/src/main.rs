// use fluvio::consumer::ConsumerConfigExtBuilder;
// use fluvio::dataplane::record::RecordData;
// use fluvio::{Fluvio, Offset, RecordKey};
// use futures::future::join;
// use serde::{Deserialize, Serialize};
// use std::fmt::Display;
// use std::time::Duration;
// use tokio::task::spawn;
// use tokio::time::timeout;
//
// const TOPIC: &str = "echo";
// const TIMEOUT_MS: u64 = 5_000;
//
// #[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
// #[serde(tag = "type")]
// pub enum Message {
//     ConnectionRefused { id: String },
//     ConnectionError,
//     InvalidRefreshCredentials,
//     Finished,
// }
//
// impl Display for Message {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Message::ConnectionRefused { id } => write!(f, "ConnectionRefused {id}"),
//             Message::ConnectionError => write!(f, "ConnectionError"),
//             Message::InvalidRefreshCredentials => write!(f, "InvalidRefreshCredentials"),
//             Message::Finished => write!(f, "Finished"),
//         }
//     }
// }
//
// impl From<Message> for RecordData {
//     fn from(message: Message) -> Self {
//         RecordData::from(serde_json::to_vec(&message).expect("serialize"))
//     }
// }
//
// #[tokio::main]
// async fn main() {
//     let produce_handle = spawn(produce());
//     let consume_handle = spawn(consume());
//
//     let timed_result = timeout(
//         Duration::from_millis(TIMEOUT_MS),
//         join(produce_handle, consume_handle),
//     )
//     .await;
//
//     let (produce_result, consume_result) = match timed_result {
//         Ok(results) => results,
//         Err(_) => {
//             println!("Echo timed out after {TIMEOUT_MS}ms");
//             std::process::exit(1);
//         }
//     };
//
//     match (produce_result, consume_result) {
//         (Err(produce_err), Err(consume_err)) => {
//             println!("Echo produce error: {produce_err:?}");
//             println!("Echo consume error: {consume_err:?}");
//             std::process::exit(1);
//         }
//         (Err(produce_err), _) => {
//             println!("Echo produce error: {produce_err:?}");
//             std::process::exit(1);
//         }
//         (_, Err(consume_err)) => {
//             println!("Echo consume error: {consume_err:?}");
//             std::process::exit(1);
//         }
//         _ => (),
//     }
// }
//
// /// Produces 10 "Hello, Fluvio" events, followed by a "Done!" event
// async fn produce() -> anyhow::Result<()> {
//     let producer = fluvio::producer(TOPIC).await?;
//
//     for i in 0..10u32 {
//         println!(
//             "Sending record {i} {}",
//             Message::ConnectionRefused { id: i.to_string() }
//         );
//         producer
//             .send(
//                 format!("Key {i}"),
//                 serde_json::to_vec(&Message::ConnectionRefused { id: i.to_string() })?,
//             )
//             .await?;
//     }
//     producer.send(RecordKey::NULL, Message::Finished).await?;
//     producer.flush().await?;
//
//     Ok(())
// }
//
// /// Consumes events until a "Done!" event is read
// async fn consume() -> anyhow::Result<()> {
//     use futures::StreamExt;
//
//     let fluvio = Fluvio::connect().await?;
//     let mut stream = fluvio
//         .consumer_with_config(
//             ConsumerConfigExtBuilder::default()
//                 .topic(TOPIC)
//                 .partition(0)
//                 .offset_start(Offset::beginning())
//                 .build()?,
//         )
//         .await?;
//
//     while let Some(Ok(record)) = stream.next().await {
//         let key = record.get_key().map(|key| key.as_utf8_lossy_string());
//         let value: Message = serde_json::from_slice(record.value()).inspect_err(|err| {
//             println!("Error decoding record: {err:?}");
//         })?;
//
//         println!("Got record: key={key:?}, value={value:?}");
//
//         if value == Message::Finished {
//             return Ok(());
//         }
//     }
//
//     Ok(())
// }
//

use anyhow::Result;
use dotenvy::dotenv;
use envconfig::Envconfig;
use integrationos_domain::telemetry::{get_subscriber, init_subscriber};
use integrationos_emit::domain::config::EmitterConfig;
use tracing::info;

fn main() -> Result<()> {
    dotenv().ok();
    let config = EmitterConfig::init_from_env()?;

    let subscriber = get_subscriber("emitter".into(), "info".into(), std::io::stdout, None);
    init_subscriber(subscriber);

    info!("Starting Emitter API with config:\n{config}");

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(config.worker_threads.unwrap_or(num_cpus::get()))
        .enable_all()
        .build()?
        .block_on(async move { todo!() })
}
