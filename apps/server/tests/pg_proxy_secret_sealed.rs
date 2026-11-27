use anybot_app_server as _;
use anybot_domain as _;
use anybot_messages as _;
use anybot_server_test as _;
use anybot_store_server as _;
use axum as _;
use clap as _;
use rpassword as _;
use serde_json as _;
use sqlx as _;
use test_harness as _;
use tracing as _;

use anybot_app_test::{
    ServerDb, db_start_with, dump_database, found_spelling, migrator_up_to, open_pool,
    public_table_names,
};
use anybot_foundation::SystemClock;
use anybot_server::{
    KEY_PROXY_SECRET, Pool, boot, product_migrations, read_config_rows, write_config_row,
};
use test_harness::pg_stop;

const PROXY_WORD: &str = "fixture-proxy-word-7d1f";

async fn boot_and_read_secret(db: &ServerDb, pool: &Pool) -> Result<Option<String>, String> {
    let mut migrations = product_migrations();
    let started = boot(
        pool,
        &SystemClock,
        &mut migrations,
        "9.9.9",
        &*db.secret_keys()?,
    )
    .await
    .map_err(|e| format!("boot rejected: {e:?}"))?;
    Ok(started
        .config
        .proxy_secret
        .map(|secret| secret.expose().to_owned()))
}

async fn plain_column(pool: &Pool) -> Result<Option<String>, String> {
    let rows = read_config_rows(pool).await.map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .find(|(key, _value)| key == KEY_PROXY_SECRET)
        .map(|(_key, value)| value))
}

async fn run_sealing(db: &ServerDb) -> Result<(), String> {
    let pool = open_pool(db).await?;
    if public_table_names(&pool)
        .await?
        .iter()
        .any(|t| t == "records")
    {
        return Err("fixture database already has the M6b migrations".to_owned());
    }
    write_config_row(&pool, KEY_PROXY_SECRET, PROXY_WORD)
        .await
        .map_err(|e| e.to_string())?;
    if boot_and_read_secret(db, &pool).await?.as_deref() != Some(PROXY_WORD) {
        return Err("first boot lost the proxy secret".to_owned());
    }
    if plain_column(&pool).await?.as_deref() != Some("") {
        return Err("plaintext proxy secret column was not cleared".to_owned());
    }
    let dump = dump_database(db)?;
    if let Some(label) = found_spelling(&dump, PROXY_WORD.as_bytes()) {
        return Err(format!("database dump holds the proxy secret ({label})"));
    }
    if boot_and_read_secret(db, &pool).await?.as_deref() != Some(PROXY_WORD) {
        return Err("second boot could not open the sealed proxy secret".to_owned());
    }
    Ok(())
}

#[tokio::test]
async fn plaintext_proxy_secret_is_sealed_at_boot_and_absent_from_a_dump() -> Result<(), String> {
    let m6a = migrator_up_to("proxy_sealed_migrations", "0002_sessions_device.sql").await?;
    let db = db_start_with("proxy_sealed", m6a).await?;
    let outcome = run_sealing(&db).await;
    let stopped = pg_stop(&db.pg).await;
    outcome.and(stopped)
}
