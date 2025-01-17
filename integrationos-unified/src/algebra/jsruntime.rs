use integrationos_domain::{ApplicationError, IntegrationOSError};
use js_sandbox_ios::Script;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::cell::RefCell;
use std::fmt::Debug;

thread_local! {
    static JS_RUNTIME: RefCell<Script> = RefCell::new(Script::new());
}

#[derive(Default, Clone, Copy)]
pub struct JSRuntimeImpl;

impl JSRuntimeImpl {
    /// Adds a JavaScript script to the runtime environment under a specific namespace.
    ///
    /// # Parameters
    ///
    /// - `fn_name`: The name of the JavaScript function to be added to the runtime.
    /// - `namespace`: The namespace
    /// - `code`: The JavaScript code to be added to the runtime.
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
    pub fn create(
        &self,
        fn_name: &str,
        namespace: &str,
        code: &str,
    ) -> Result<Self, IntegrationOSError> {
        JS_RUNTIME
            .with_borrow_mut(|script| script.add_script(namespace, fn_name, code))
            .map_err(|e| {
                tracing::error!(
                    "Failed to create javascript function in namespace {}. Error: {}",
                    namespace,
                    e
                );

                ApplicationError::bad_request(
                    &format!("Failed while creating request schema mapping script: {e}"),
                    None,
                )
            })?;

        Ok(*self)
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
    pub async fn run<P, R>(&self, payload: &P, namespace: &str) -> Result<R, IntegrationOSError>
    where
        P: Serialize + Debug,
        R: DeserializeOwned + Debug,
    {
        let payload = serde_json::to_value(payload).map_err(|e| {
            tracing::error!("Error serializing payload: {}", e);

            ApplicationError::bad_request(
                &format!("Failed while serializing request for request schema mapping script: {e}"),
                None,
            )
        })?;

        let payload = JS_RUNTIME
            .with_borrow_mut(|script| script.call_namespace(namespace, payload))
            .map_err(|e| {
                tracing::error!("Error running javascript function: {}", e);

                ApplicationError::bad_request(
                    &format!("Failed while running request schema mapping script: {e}"),
                    None,
                )
            });

        tokio::task::yield_now().await;

        payload
    }
}
