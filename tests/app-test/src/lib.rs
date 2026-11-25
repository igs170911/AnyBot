use anybot_foundation as _;
use anybot_messages::{Catalog, Lang, MessageCode, Params};
use anybot_server as _;
use sqlx::Row as _;
use tokio as _;
use tracing as _;

use test_harness::{Program, Running, migration_fixture, pg_start, pg_stop};

pub mod clockwalk;
pub mod http;
pub mod serve;
pub mod serverdb;

pub use clockwalk::{ClockWalker, WAVE_FAKE_STEP, settle_handles};
pub use serverdb::{ServerDb, db_start};

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
