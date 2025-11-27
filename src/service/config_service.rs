// src/service/config_service.rs
use crate::common::error::Result;
use crate::common::types::SystemConfig;
use crate::driver::storage::StorageDriver;

pub struct ConfigService<S: StorageDriver> {
    storage: S,
}

impl<S: StorageDriver> ConfigService<S> {
    pub fn new(storage: S) -> Self {
        Self { storage }
    }

    pub async fn load_config(&mut self) -> Result<SystemConfig> {
        self.storage.load_config().await
    }

    pub async fn save_config(&mut self, config: &SystemConfig) -> Result<()> {
        self.storage.save_config(config).await
    }

    pub async fn reset_to_defaults(&mut self) -> Result<()> {
        let default_config = SystemConfig::default();
        self.save_config(&default_config).await
    }
}
