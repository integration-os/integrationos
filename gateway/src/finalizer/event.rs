use async_trait::async_trait;
use integrationos_domain::{encrypted_access_key::EncryptedAccessKey, Event};

#[async_trait]
pub trait FinalizeEvent {
    async fn finalize_event(
        &self,
        event: &Event,
        event_name: &str,
        access_key: &EncryptedAccessKey,
    ) -> Result<String, anyhow::Error>;
}
