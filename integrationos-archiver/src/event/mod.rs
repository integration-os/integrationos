pub mod completed;
pub mod dumped;
pub mod failed;
pub mod started;

use chrono::NaiveDate;
use completed::Completed;
use dumped::Dumped;
use failed::Failed;
use integrationos_domain::Id;
use serde::{Deserialize, Serialize};
use started::Started;

pub trait EventMetadata {
    fn reference(&self) -> Id;
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Event {
    Started(Started),
    Dumped(Dumped),
    Failed(Failed),
    Completed(Completed),
}

impl Event {
    pub fn date(&self) -> NaiveDate {
        match self {
            Event::Started(e) => e.date(),
            Event::Dumped(e) => e.date(),
            Event::Failed(e) => e.date(),
            Event::Completed(e) => e.date(),
        }
    }
}
