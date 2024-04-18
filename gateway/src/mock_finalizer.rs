use super::finalize_event::FinalizeEvent;
use async_trait::async_trait;
use integrationos_domain::{encrypted_access_key::EncryptedAccessKey, Event};

pub struct MockFinalizer;

#[async_trait]
impl FinalizeEvent for MockFinalizer {
    async fn finalize_event(
        &self,
        _event: &Event,
        _event_name: &str,
        _access_key: &EncryptedAccessKey,
    ) -> Result<String, anyhow::Error> {
        Ok("sent".to_owned())
    }
}
