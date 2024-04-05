use super::finalize_event::FinalizeEvent;
use crate::config::Config;
use integrationos_domain::common::{
    encrypted_access_key::EncryptedAccessKey, encrypted_data::PASSWORD_LENGTH, AccessKey,
};
use moka::future::Cache;
use std::sync::Arc;

pub struct AppState {
    pub config: Config,
    pub cache: Cache<EncryptedAccessKey<'static>, AccessKey>,
    pub finalizer: Arc<dyn FinalizeEvent + Sync + Send>,
}

impl AppState {
    pub fn new(config: Config, finalizer: Arc<dyn FinalizeEvent + Sync + Send>) -> Self {
        let cache = Cache::new(config.cache_size);
        Self {
            config,
            cache,
            finalizer,
        }
    }

    pub fn get_secret_key(&self) -> [u8; PASSWORD_LENGTH] {
        // We validate that the config must have 32 byte secret key in main.rs
        // So this is safe to unwrap
        self.config.secret_key.as_bytes().try_into().unwrap()
    }
}
