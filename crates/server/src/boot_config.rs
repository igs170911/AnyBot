use anybot_foundation::Secret;
use anybot_store_server::config_sealed::{config_place, read_sealed_config, store_sealed_config};
use anybot_store_server::{Pool, SecretKeys, read_config_rows};

use crate::boot::BootError;
use crate::config::{KEY_PROXY_SECRET, ServerConfig};

pub async fn load_config(pool: &Pool, keys: &SecretKeys) -> Result<ServerConfig, BootError> {
    seal_plain_proxy_secret(pool, keys).await?;
    let rows = read_config_rows(pool).await.map_err(read_failed)?;
    let mut config = ServerConfig::from_rows(rows).map_err(BootError::ConfigInvalid)?;
    config.proxy_secret = open_proxy_secret(pool, keys).await?;
    Ok(config)
}

async fn seal_plain_proxy_secret(pool: &Pool, keys: &SecretKeys) -> Result<(), BootError> {
    let rows = read_config_rows(pool).await.map_err(read_failed)?;
    let Some((_key, value)) = rows
        .into_iter()
        .find(|(key, value)| key == KEY_PROXY_SECRET && !value.is_empty())
    else {
        return Ok(());
    };
    let plain = Secret::from_untrusted(value);
    let sealed = keys
        .seal_text(&config_place(KEY_PROXY_SECRET), &plain)
        .map_err(|e| BootError::ConfigInvalid(format!("{KEY_PROXY_SECRET}: {e}")))?;
    store_sealed_config(pool, KEY_PROXY_SECRET, &sealed)
        .await
        .map_err(read_failed)
}

async fn open_proxy_secret(pool: &Pool, keys: &SecretKeys) -> Result<Option<Secret>, BootError> {
    let Some(sealed) = read_sealed_config(pool, KEY_PROXY_SECRET)
        .await
        .map_err(read_failed)?
    else {
        return Ok(None);
    };
    keys.open_text(&config_place(KEY_PROXY_SECRET), &sealed)
        .map(Some)
        .map_err(|e| BootError::ConfigInvalid(format!("{KEY_PROXY_SECRET}: {e}")))
}

fn read_failed(err: impl std::fmt::Display) -> BootError {
    BootError::ConfigRead(err.to_string())
}
