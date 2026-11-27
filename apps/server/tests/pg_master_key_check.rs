use anybot_app_server as _;
use anybot_domain as _;
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
    ServerDb, ServerProgram, bootstrap_text, db_start, english, fixture_migrator, open_pool,
    public_table_names,
};
use anybot_foundation::{MasterKey, SystemClock};
use anybot_messages::{Catalog, MessageCode, Params};
use anybot_server::{
    BootError, KEY_SERVER_VERSION, Pool, SecretKeys, boot, product_migrations, read_config_rows,
};
use test_harness::{pg_stop, run_program, write_file_with_mode};

fn server_bin() -> ServerProgram {
    ServerProgram::new(env!("CARGO_BIN_EXE_anybot-server"))
}

fn other_keys() -> Result<SecretKeys, String> {
    let key = MasterKey::generate().map_err(|e| e.to_string())?;
    SecretKeys::from_master(&key).map_err(|e| e.to_string())
}

async fn snapshot(pool: &Pool) -> Result<(Vec<String>, Vec<(String, String)>), String> {
    let tables = public_table_names(pool).await?;
    let rows = read_config_rows(pool).await.map_err(|e| e.to_string())?;
    Ok((tables, rows))
}

async fn first_boot(db: &ServerDb, pool: &Pool) -> Result<(), String> {
    let mut migrations = product_migrations();
    boot(
        pool,
        &SystemClock,
        &mut migrations,
        "9.9.9",
        &*db.secret_keys()?,
    )
    .await
    .map(|_started| ())
    .map_err(|e| format!("first boot rejected: {e:?}"))
}

async fn run_wrong_key_upgrade(db: &ServerDb) -> Result<(), String> {
    let pool = open_pool(db).await?;
    first_boot(db, &pool).await?;
    let before = snapshot(&pool).await?;
    let mut upgraded = fixture_migrator("0003_fixture_extra.sql").await?;
    let result = boot(&pool, &SystemClock, &mut upgraded, "9.9.10", &other_keys()?).await;
    if !matches!(result, Err(BootError::MasterKeyMismatch)) {
        return Err(format!("wrong key boot gave {result:?}"));
    }
    let after = snapshot(&pool).await?;
    if after != before {
        return Err(format!(
            "wrong key changed the database: {before:?} -> {after:?}"
        ));
    }
    if after.0.iter().any(|name| name == "fixture_extra") {
        return Err("pending migration ran under the wrong key".to_owned());
    }
    Ok(())
}

#[tokio::test]
async fn wrong_master_key_stops_before_any_migration() -> Result<(), String> {
    let db = db_start("master_key_before_migrations").await?;
    let outcome = run_wrong_key_upgrade(&db).await;
    let stopped = pg_stop(&db.pg).await;
    outcome.and(stopped)
}

async fn run_wrong_key_binary(db: &ServerDb) -> Result<(), String> {
    let pool = open_pool(db).await?;
    first_boot(db, &pool).await?;
    let version_before = version_row(&pool).await?;
    let wrong = MasterKey::generate().map_err(|e| e.to_string())?;
    write_file_with_mode(&db.bootstrap, &bootstrap_text(&db.url, &wrong), 0o600)?;
    let outcome = run_program(&server_bin().program(db.args()), &db.dir)?;
    if outcome.code != 2 {
        return Err(format!("wrong key exit {}: {outcome:?}", outcome.code));
    }
    let want = english(
        &mut Catalog::new(),
        MessageCode::ServerMasterKeyMismatch,
        &Params::new(),
    )?;
    if !outcome.stdout.contains(&want) {
        return Err(format!("wrong key output misses {want}: {outcome:?}"));
    }
    if version_row(&pool).await? != version_before {
        return Err("wrong key boot rewrote the server version".to_owned());
    }
    Ok(())
}

async fn version_row(pool: &Pool) -> Result<Option<String>, String> {
    let rows = read_config_rows(pool).await.map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .find(|(key, _value)| key == KEY_SERVER_VERSION)
        .map(|(_key, value)| value))
}

#[tokio::test]
async fn wrong_master_key_exits_two_from_the_binary() -> Result<(), String> {
    let db = db_start("master_key_binary").await?;
    let outcome = run_wrong_key_binary(&db).await;
    let stopped = pg_stop(&db.pg).await;
    outcome.and(stopped)
}
