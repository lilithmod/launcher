use std::{fs, sync::Arc};

use futures::TryFutureExt;
use log::{error, info};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::utils::{
    config::ConfigError::{Save, Serialization},
    files::config_path,
};

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct LauncherConfig {
    pub alpha: bool,
    pub debug: bool,
}

pub fn init_config() -> Arc<RwLock<LauncherConfig>> {
    let mut config: LauncherConfig = LauncherConfig::default();
    if let Ok(data) = fs::read_to_string(config_path()) {
        config = serde_json::from_str::<LauncherConfig>(&data).unwrap_or_default();
        info!(target:"init_config","loaded config: {config:?}");
    }
    Arc::new(RwLock::new(config))
}

pub enum ConfigError {
    Serialization,
    Save,
}

/// needs to be run in a tokio runtime
pub async fn save_config(config: LauncherConfig) -> Result<(), ConfigError> {
    let serialized = serde_json::to_string_pretty(&config).map_err(|e| {
        error!(target:"save_config", "serialization failed: {e}");
        Serialization
    })?;
    tokio::fs::write(config_path(), serialized)
        .map_err(|e| {
            error!(target:"save_config", "write failed: {e}");
            Save
        })
        .await?;
    info!(target:"save_config", "wrote new config: {config:?}");

    Ok(())
}
