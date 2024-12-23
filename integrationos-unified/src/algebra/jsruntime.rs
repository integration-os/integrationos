use derive_builder::Builder;
use integrationos_domain::{ApplicationError, IntegrationOSError};
use js_sandbox_ios::Script;
use serde::Serialize;
use serde_json::Value;
use std::cell::RefCell;
use serde::de::DeserializeOwned;

thread_local! {
    static JS_RUNTIME: RefCell<Script> = RefCell::new(Script::new());
}

#[derive(Builder)]
#[builder(setter(into), build_fn(error = "IntegrationOSError"))]
pub struct JSRuntimeImpl {
    namespace: String,
    code: String,
}

impl JSRuntimeImpl {
    pub fn create(self, fn_name: &str) -> Result<Self, IntegrationOSError> {
        JS_RUNTIME
            .with_borrow_mut(|script| script.add_script(&self.namespace, fn_name, &self.code))
            .map_err(|e| {
                tracing::error!(
                    "Failed to create request schema mapping script. ID: {}, Error: {}",
                    self.namespace,
                    e
                );

                ApplicationError::bad_request(
                    &format!("Failed while creating request schema mapping script: {e}"),
                    None,
                )
            })?;

        Ok(self)
    }

    pub async fn run<P, R>(&self, body: &P) -> Result<R, IntegrationOSError>
    where
        P: Serialize,
        R: DeserializeOwned,
    {
        let body = serde_json::to_value(body).map_err(|e| {
            tracing::error!(
                "Failed to serialize request for request schema mapping script for connection model. Error: {}",
                e
            );

            ApplicationError::bad_request(
                &format!("Failed while serializing request for request schema mapping script: {e}"),
                None,
            )
        })?;
        
        let body = JS_RUNTIME
            .with_borrow_mut(|script| script.call_namespace(&self.namespace, body))
            .map_err(|e| {
                tracing::error!(
                    "Failed to run request schema mapping script for connection model. Error: {}",
                    e
                );

                ApplicationError::bad_request(
                    &format!("Failed while running request schema mapping script: {e}"),
                    None,
                )
            });

        tokio::task::yield_now().await;

        body
    }
}
