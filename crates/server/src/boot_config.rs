use anybot_store_server::{Pool, SecretKeys, read_config_rows};

use crate::boot::BootError;
use crate::config::ServerConfig;

pub async fn load_config(pool: &Pool, _keys: &SecretKeys) -> Result<ServerConfig, BootError> {
    let rows = read_config_rows(pool)
        .await
        .map_err(|e| BootError::ConfigRead(e.to_string()))?;
    ServerConfig::from_rows(rows).map_err(BootError::ConfigInvalid)
}
