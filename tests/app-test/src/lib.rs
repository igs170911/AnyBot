use anybot_foundation as _;
use anybot_messages::{Catalog, Lang, MessageCode, Params};
use anybot_server as _;
use sqlx::Row as _;
use tokio as _;
use tracing as _;

use test_harness::{Program, Running, migration_fixture, pg_start, pg_stop};

pub mod clockwalk;
pub mod db_dump;
pub mod fsutil;
pub mod http;
pub mod install_live;
pub mod n106;
pub mod serve;
pub mod serverdb;
pub mod startup;

pub mod fake_totp;
pub mod log_capture;
pub mod mailbox_audit;
pub mod mailbox_model;
#[cfg(test)]
mod mailbox_model_tests;
pub mod mailbox_ops;
pub mod mailbox_prop;
pub mod mailbox_wait;
pub mod mailbox_x;
pub mod sse;
pub mod sse_client;
pub mod sse_watch;
pub mod two_step_client;
pub use install_live::{
    bootstrap_args, check_both, check_second, finish_running, get_status, public_url_check,
    start_live, startup_params, stop_live, wait_health,
};

pub use clockwalk::{ClockWalker, WAVE_FAKE_STEP, WAVE_INSERT_STEP, settle_handles};
pub use db_dump::{
    comparable_dump, dump_database, found_digest, found_raw_bytes, found_spelling, migrator_up_to,
    unkeyed_sha256,
};
pub use serverdb::{ServerDb, bootstrap_text, db_start, db_start_with};

pub fn english(
    catalog: &mut Catalog,
    code: MessageCode,
    params: &Params,
) -> Result<String, String> {
    catalog
        .format(Lang::En, &code, params)
        .map_err(|e| e.to_string())
}

#[must_use]
pub fn migrations_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../migrations/server")
}

pub async fn fixture_migrator(extra: &str) -> Result<anybot_server::Migrator, String> {
    let dir = migration_fixture(&migrations_dir(), extra, extra)?;
    anybot_server::Migrator::new(dir)
        .await
        .map_err(|e| e.to_string())
}

pub async fn open_pool(db: &ServerDb) -> Result<anybot_server::Pool, String> {
    anybot_server::open_url(&db.url)
        .await
        .map_err(|e| e.to_string())
}

pub async fn public_table_names(pool: &anybot_server::Pool) -> Result<Vec<String>, String> {
    let rows = sqlx::query(
        "SELECT tablename FROM pg_catalog.pg_tables WHERE schemaname = 'public' ORDER BY tablename",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    let mut names = Vec::new();
    for row in rows {
        names.push(
            row.try_get::<String, _>("tablename")
                .map_err(|e| e.to_string())?,
        );
    }
    Ok(names)
}

pub async fn blocked_database_fixture(
    test: &str,
) -> Result<(ServerDb, std::path::PathBuf), String> {
    let db = db_start(test).await?;
    let dir = db.dir.clone();
    db.pg.pause().await?;
    Ok((db, dir))
}

pub async fn clean_fixture_database(
    test: &str,
) -> Result<(test_harness::PgInstance, String, std::path::PathBuf), String> {
    let dir = test_harness::test_dir(test)?;
    let pg = pg_start(&dir).await?;
    let _created = pg.create_database("anybot_test").await;
    let url = pg.url_for("anybot_test");
    Ok((pg, url, dir))
}

pub async fn stop_fixture_database(pg: &test_harness::PgInstance) -> Result<(), String> {
    pg_stop(pg).await
}

#[derive(Default)]
pub struct ServerProgram {
    pub bin: String,
}

impl ServerProgram {
    #[must_use]
    pub fn new(bin: &str) -> Self {
        ServerProgram {
            bin: bin.to_owned(),
        }
    }

    #[must_use]
    pub fn program(&self, args: Vec<String>) -> Program {
        Program::new(&self.bin, args)
    }

    pub fn start_running(&self, db: &ServerDb, expected: &str) -> Result<Running, String> {
        let program = self.program(db.args());
        let mut running = Running::start(&program, &db.dir)?;
        let announced = running.next_stdout_line()?;
        if announced.trim() != expected {
            let _result = running.kill();
            return Err(format!("startup line {announced} differs from {expected}"));
        }
        Ok(running)
    }
}
