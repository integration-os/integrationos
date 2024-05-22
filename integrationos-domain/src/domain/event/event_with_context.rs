use super::Event;
use crate::RootContext;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventWithContext {
    pub event: Event,
    pub context: RootContext,
}

impl EventWithContext {
    pub fn new(event: Event, context: RootContext) -> Self {
        Self { event, context }
    }
}
