use crate::id::Id;
use serde::{Deserialize, Serialize};

use super::{event_state::EventState, hashes::HashValue, Event};

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EventResponse {
    pub status: EventState,
    pub key: Id,
    pub payload_byte_length: usize,
    pub hashes: [HashValue; 3],
}

impl EventResponse {
    pub fn new(event: Event) -> Self {
        Self {
            status: EventState::Acknowledged,
            key: event.key,
            payload_byte_length: event.payload_byte_length,
            hashes: event.hashes,
        }
    }
}
