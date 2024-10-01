pub mod completed;
pub mod dumped;
pub mod failed;
pub mod finished;
pub mod started;
pub mod uploaded;

use completed::Completed;
use dumped::Dumped;
use failed::Failed;
use finished::Finished;
use integrationos_domain::Id;
use serde::{Deserialize, Serialize};
use started::Started;
use uploaded::Uploaded;

pub trait EventMetadata {
    fn reference(&self) -> Id;
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum Event {
    Started(Started),
    Dumped(Dumped),
    Failed(Failed),
    Uploaded(Uploaded),
    Completed(Completed),
    Finished(Finished),
}
