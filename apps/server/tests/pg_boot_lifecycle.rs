use std::sync::Arc;

use anybot_app_server as _;
use anybot_app_test as _;
use anybot_app_test as _;
use anybot_app_test::{
    ServerProgram, db_start, english, fixture_migrator, open_pool, public_table_names,
};
use anybot_foundation as _;
use anybot_foundation::SystemClock;
use anybot_messages::{Catalog, MessageCode, Params};
use anybot_server::{
    DEFAULT_PORT, Pool, ServerConfig, boot, product_migrations, read_config_rows, set_config_rows,
    write_config_row,
};
use axum as _;
use clap as _;
use serde_json as _;
use sqlx as _;
use test_harness as _;
use tracing as _;

use test_harness::{pg_stop, run_program};

fn server_bin() -> ServerProgram {
    ServerProgram::new(env!("CARGO_BIN_EXE_anybot-server"))
}

#[tokio::test]
async fn t_server_boot_3() -> Result<(), String> {
    let db = db_start("server_boot_3").await?;
    let pool = open_pool(&db).await?;
    let clock = Arc::new(SystemClock);
    let first = {
        let mut m = product_migrations();
        boot(&pool, clock.as_ref(), &mut m, "9.9.9").await
    };
    let upgraded = fixture_migrator("0002_fixture_extra.sql").await;
    let outcome = match first {
        Ok(_started) => match upgraded {
            Ok(mut migrator) => boot_up_and_down(&pool, clock.as_ref(), &mut migrator).await,
            Err(msg) => Err(msg),
        },
        Err(err) => Err(format!("product boot rejected: {err:?}")),
    };
    let _result = db.stop().await;
    outcome
}

async fn boot_up_and_down(
    pool: &Pool,
    clock: &SystemClock,
    upgraded: &mut anybot_server::Migrator,
) -> Result<(), String> {
    let up = boot(pool, clock, upgraded, "9.9.10").await;
    match up {
        Ok(_started) => {}
        Err(err) => {
            return Err(format!("upgraded boot rejected: {err:?}"));
        }
    }
    let names = public_table_names(pool).await?;
    if !names.iter().any(|n| n == "fixture_extra") {
        return Err(format!("upgrade added no table: {names:?}"));
    }
    let back = {
        let mut m = product_migrations();
        boot(pool, clock, &mut m, "9.9.9").await
    };
    match back {
        Ok(_started) => {}
        Err(err) => {
            return Err(format!("downgraded boot rejected: {err:?}"));
        }
    }
    let kept = public_table_names(pool).await?;
    if !kept.iter().any(|n| n == "fixture_extra") {
        return Err(format!("downgrade lost data: {kept:?}"));
    }
    Ok(())
}

#[tokio::test]
async fn migration_failure_keeps_the_schema_and_the_old_set_boots() -> Result<(), String> {
    let db = db_start("boot_2_failure").await?;
    let pool = open_pool(&db).await?;
    let clock = Arc::new(SystemClock);
    let before = public_table_names(&pool).await?;
    let broken = fixture_migrator("0002_fixture_broken.sql").await;
    let outcome = run_broken_migration(&pool, clock.as_ref(), broken, &before).await;
    let old = {
        let mut m = product_migrations();
        boot(&pool, clock.as_ref(), &mut m, "9.9.9").await
    };
    let _result = db.stop().await;
    match old {
        Ok(_started) => outcome,
        Err(err) => Err(format!("old set rejected after failure: {err:?}")),
    }
}

async fn run_broken_migration(
    pool: &Pool,
    clock: &SystemClock,
    broken: Result<anybot_server::Migrator, String>,
    before: &[String],
) -> Result<(), String> {
    let mut migrator = match broken {
        Ok(m) => m,
        Err(msg) => return Err(msg),
    };
    let result = boot(pool, clock, &mut migrator, "9.9.10").await;
    if result.is_ok() {
        return Err("broken migration was accepted".to_owned());
    }
    let after = public_table_names(pool).await?;
    if after != before {
        return Err(format!("schema changed after failure: {after:?}"));
    }
    Ok(())
}

#[tokio::test]
async fn t_account_bind_4() -> Result<(), String> {
    let db = db_start("account_bind_4").await?;
    let pool = open_pool(&db).await?;
    set_config_rows(
        &pool,
        &[("listen_addresses".to_owned(), "abc".to_owned())],
        &[],
    )
    .await
    .map_err(|e| e.to_string())?;
    let outcome = run_program(&server_bin().program(db.args()), &db.dir)?;
    let _result = pg_stop(&db.pg).await;
    let code = i64::from(outcome.code);
    if code != 1 {
        return Err(format!("invalid-address exit {}", outcome.code));
    }
    let expected = english(
        &mut Catalog::new(),
        MessageCode::ServerListenAddressInvalid,
        &Params::new()
            .with_text("address", "abc")
            .with_text("reason", "not an IP address"),
    )?;
    if !outcome.stdout.contains(&expected) {
        return Err(format!("{outcome:?} misses {expected}"));
    }
    if outcome.stdout.contains("listening on") {
        return Err("server announced listening despite the bad address".to_owned());
    }
    Ok(())
}

#[tokio::test]
async fn config_defaults_and_public_url_round_trip_through_the_database() -> Result<(), String> {
    let db = db_start("config_rows").await?;
    let pool = open_pool(&db).await?;
    let rows = read_config_rows(&pool).await.map_err(|e| e.to_string())?;
    let config = ServerConfig::from_rows(rows.clone());
    let _written = write_config_row(&pool, "public_url", "https://round.example/")
        .await
        .map_err(|e| e.to_string());
    let after = read_config_rows(&pool).await.map_err(|e| e.to_string())?;
    let _result = db.stop().await;
    check_config_rows(&config, &rows, &after)
}

fn check_config_rows(
    config: &ServerConfig,
    rows: &[(String, String)],
    after: &[(String, String)],
) -> Result<(), String> {
    if config.listen_port == DEFAULT_PORT && !rows.is_empty() {
        return Err(format!("fixture rows missing: {rows:?}"));
    }
    if config.listen_port == DEFAULT_PORT {
        return Err("fixture port equals the default".to_owned());
    }
    if !after
        .iter()
        .any(|(k, v)| k == "public_url" && v == "https://round.example/")
    {
        return Err(format!("public_url not stored: {after:?}"));
    }
    Ok(())
}
