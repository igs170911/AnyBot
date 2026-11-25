#![forbid(unsafe_code)]

use aes_gcm as _;
use anybot_foundation as _;

use axum as _;
use base64 as _;
use bytes as _;
use getrandom as _;
use hkdf as _;
use libc as _;
use p256 as _;
use postgresql_embedded as _;
use reqwest as _;
use serde as _;
use serde_json as _;
use sha2 as _;
use socket2 as _;
use tokio_stream as _;
use toml as _;
use tracing as _;
use tracing_subscriber as _;
use url as _;

use std::path::PathBuf;

use sqlx::migrate::Migrator;

use test_harness::{pg_start, pg_stop, repo_root, test_dir};

async fn seed(url_file: &PathBuf, pid_file: &PathBuf) -> Result<(), String> {
    let dir = test_dir("sqlx_seed")?;
    let pg = pg_start(&dir).await?;
    pg.create_database("anybot_prepare").await?;
    let url = pg.url_for("anybot_prepare");
    let pool = sqlx::PgPool::connect(&url)
        .await
        .map_err(|e| e.to_string())?;
    let migrations = repo_root().join("migrations").join("server");
    let migrator = Migrator::new(migrations).await.map_err(|e| e.to_string())?;
    migrator.run(&pool).await.map_err(|e| e.to_string())?;
    pool.close().await;
    std::fs::write(url_file, format!("{url}\n")).map_err(|e| e.to_string())?;
    std::fs::write(pid_file, format!("{}\n", std::process::id())).map_err(|e| e.to_string())?;
    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .map_err(|e| e.to_string())?
        .recv()
        .await;
    pg_stop(&pg).await
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let url_file = flag_path(&argv, "--url-file")?;
    let pid_file = flag_path(&argv, "--pid-file")?;
    seed(&url_file, &pid_file).await
}

fn flag_path(argv: &[String], name: &str) -> Result<PathBuf, String> {
    let mut seen = false;
    for arg in argv {
        if seen {
            return Ok(PathBuf::from(arg));
        }
        if arg == name {
            seen = true;
        }
    }
    Err(format!("missing value for {name}"))
}

#[cfg(test)]
mod link_dev_deps {
    use anybot_store_server as _;
    use sad_web_push as _;
    use sqlx as _;
}
