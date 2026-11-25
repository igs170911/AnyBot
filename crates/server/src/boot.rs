use anybot_foundation::{Clock, MessageRef, ServerBootstrap, TimestampMs};
use anybot_store_server::Migrator;

use anybot_store_server::{
    Pool, SchemaError, account_count, ensure_schema, read_config_rows, write_config_row,
};

use crate::config::KEY_SERVER_VERSION;
use crate::config::ServerConfig;

#[derive(Debug)]
pub struct Started {
    pub config: ServerConfig,
    pub account_count: i64,
    pub new_invite: Option<String>,
}

impl Started {
    #[must_use]
    pub fn startup_line(
        &self,
        version: &str,
        addresses: &str,
        bootstrap: &ServerBootstrap,
    ) -> MessageRef {
        MessageRef::new(crate::startup::CODE_SERVER_STARTUP_LISTENING)
            .with_text("version", version)
            .with_text("addresses", addresses)
            .with_text("db_host", &bootstrap.host)
            .with_text("db_name", &bootstrap.database)
    }
}

#[derive(Debug)]
pub enum BootError {
    Schema(SchemaError),
    ConfigRead(String),
    ConfigInvalid(String),
    VersionWrite(String),
}

pub async fn boot(
    pool: &Pool,
    clock: &dyn Clock,
    migrator: &mut Migrator,
    version: &str,
) -> Result<Started, BootError> {
    ensure_schema(pool, migrator)
        .await
        .map_err(BootError::Schema)?;
    write_version(pool, version, clock.now_ms()).await?;
    let rows = read_config_rows(pool)
        .await
        .map_err(|e| BootError::ConfigRead(e.to_string()))?;
    let mut config = ServerConfig::from_rows(rows).map_err(BootError::ConfigInvalid)?;
    let account_count = account_count(pool)
        .await
        .map_err(|e| BootError::ConfigRead(e.to_string()))?;
    let new_invite = ensure_invite(pool, &mut config).await?;
    Ok(Started {
        config,
        account_count,
        new_invite,
    })
}

async fn ensure_invite(
    pool: &Pool,
    config: &mut ServerConfig,
) -> Result<Option<String>, BootError> {
    if config.invite_code.is_some() {
        return Ok(None);
    }
    let code = crate::invite::draw_invite().map_err(BootError::ConfigRead)?;
    write_config_row(pool, crate::config::KEY_INVITE_CODE, &code)
        .await
        .map_err(|e| BootError::ConfigRead(e.to_string()))?;
    config.invite_code = Some(anybot_foundation::Secret::from_untrusted(code.clone()));
    Ok(Some(code))
}

async fn write_version(pool: &Pool, version: &str, at: TimestampMs) -> Result<(), BootError> {
    let text = format!("{version}@{}", at.millis());
    write_config_row(pool, KEY_SERVER_VERSION, &text)
        .await
        .map_err(|e| BootError::VersionWrite(e.to_string()))
}
