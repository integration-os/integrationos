use super::EventMetadata;
use integrationos_domain::{prefix::IdPrefix, Id};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DateChosen {
    #[serde(rename = "_id")]
    id: Id,
    reference: Id,
    starts_from: i64,
    ends_at: i64,
}

impl DateChosen {
    pub fn new(reference: Id, starts_from: i64, ends_at: i64) -> Self {
        Self {
            id: Id::now(IdPrefix::Archive),
            reference,
            starts_from,
            ends_at,
        }
    }

    pub fn event_date(&self) -> i64 {
        self.ends_at
    }
}

impl EventMetadata for DateChosen {
    fn reference(&self) -> Id {
        self.reference
    }
}
