use derive_builder::Builder;
use integrationos_domain::{ApplicationError, IntegrationOSError};
use js_sandbox_ios::Script;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::cell::RefCell;

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
    /// Adds a JavaScript script to the runtime environment under a specific namespace.
    ///
    /// # Parameters
    ///
    /// - `fn_name`: The name of the JavaScript function to be added to the runtime.
    ///
    /// # Returns
    ///
    /// A `Result` containing:
    /// - `Self`: On success, the instance of the struct.
    /// - `IntegrationOSError`: On failure, encapsulates the error details.
    ///
    /// # Errors
    ///
    /// Returns an error if adding the script to the runtime fails, logging the error
    /// and returning a `bad_request` application error.
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

    /// Executes a JavaScript function in the runtime associated with a specific namespace,
    /// passing serialized input data and deserializing the output.
    ///
    /// # Parameters
    ///
    /// - `body`: The input data to be serialized and passed to the JavaScript function.
    ///
    /// # Type Parameters
    ///
    /// - `P`: The type of the input data. Must implement `Serialize`.
    /// - `R`: The type of the output data. Must implement `DeserializeOwned`.
    ///
    /// # Returns
    ///
    /// A `Result` containing:
    /// - `R`: The deserialized output of the JavaScript function on success.
    /// - `IntegrationOSError`: On failure, encapsulates the error details.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization of the input data fails or if the JavaScript
    /// function fails to execute. Logs the error and returns a `bad_request` application error.
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
