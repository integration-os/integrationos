use crate::store::ContextStore;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use integrationos_domain::{algebra::execution::ExecutionContext, id::Id};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

type Contexts = Arc<Mutex<HashMap<Id, Vec<Box<dyn ExecutionContext>>>>>;

#[derive(Clone, Default)]
pub struct MockContextStorage {
    pub contexts: Contexts,
}

impl MockContextStorage {
    pub fn new() -> Self {
        Self {
            contexts: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl ContextStore for MockContextStorage {
    async fn get<T: ExecutionContext + Clone + for<'a> Deserialize<'a> + Unpin>(
        &self,
        context_key: &Id,
    ) -> Result<T> {
        self.contexts
            .lock()
            .unwrap()
            .get(context_key)
            .map(|c| {
                let last = c.last();
                last.expect("No context for {context_key}")
                    .downcast_ref::<T>()
                    .expect("ExecutionContext could not be downcast")
                    .clone()
            })
            .ok_or(anyhow!("No context for {context_key}"))
    }

    async fn set<T: ExecutionContext + Clone + Serialize>(&self, context: T) -> Result<()> {
        let context = Box::new(context);
        self.contexts
            .lock()
            .unwrap()
            .entry(*context.context_key())
            .and_modify(|v| v.push(context.clone()))
            .or_insert(vec![context]);
        Ok(())
    }
}
