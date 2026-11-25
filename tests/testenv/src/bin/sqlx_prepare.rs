#![forbid(unsafe_code)]

use anybot_foundation as _;
use anybot_server as _;
use axum as _;
use serde as _;
use serde_json as _;
use testenv as _;
use tokio as _;

use std::time::Duration;

use anybot_server::{ensure_schema, open_url, product_migrations};

use test_harness::{Program, pg_start, pg_stop, repo_root, run_program, test_dir};

const PREPARE_TIMEOUT: Duration = Duration::from_mins(15);

fn prepare_args(check: bool) -> Vec<String> {
    let mut args = vec![
        "sqlx".to_owned(),
        "prepare".to_owned(),
        "--workspace".to_owned(),
        "--all".to_owned(),
    ];
    if check {
        args.push("--check".to_owned());
    }
    args.push("--".to_owned());
    args.push("--all-targets".to_owned());
    args
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let cargo = cargo_path(&argv)?;
    let check = argv.iter().any(|a| a == "--check");
    let dir = test_dir("sqlx_prepare")?;
    let pg = pg_start(&dir).await?;
    pg.create_database("anybot_prepare").await?;
    let url = pg.url_for("anybot_prepare");
    let outcome = run_prepare(&url, &cargo, check).await;
    let stopped = pg_stop(&pg).await;
    finish(outcome, stopped)
}

fn cargo_path(argv: &[String]) -> Result<String, String> {
    let joined = argv.join(" ");
    let parts: Vec<&str> = joined.split("--cargo ").skip(1).collect();
    let head = parts.first().ok_or("missing --cargo path argument")?;
    let word = head
        .split_whitespace()
        .next()
        .ok_or("missing --cargo path value")?;
    Ok(word.to_owned())
}

async fn run_prepare(url: &str, cargo: &str, check: bool) -> Result<test_harness::Outcome, String> {
    let prepared = prepare_schema(url).await;
    let program = Program::new(cargo, prepare_args(check))
        .with_env("DATABASE_URL", url)
        .with_env("SQLX_OFFLINE", "false")
        .with_timeout(PREPARE_TIMEOUT);
    let result = run_program(&program, &repo_root());
    match prepared {
        Ok(()) => result,
        Err(msg) => Err(format!("seed failed: {msg}; prepare run {result:?}")),
    }
}

fn finish(
    outcome: Result<test_harness::Outcome, String>,
    stopped: Result<(), String>,
) -> Result<(), String> {
    match outcome {
        Ok(running) => match stopped {
            Ok(()) => {
                if running.code == 0 {
                    Ok(())
                } else {
                    Err(format!(
                        "sqlx prepare exited {}: stdout {} stderr {}",
                        running.code, running.stdout, running.stderr
                    ))
                }
            }
            Err(msg) => Err(format!(
                "pg stop failed: {msg}; prepare code {} stdout {} stderr {}",
                running.code, running.stdout, running.stderr
            )),
        },
        Err(msg) => Err(format!("{msg}; pg stop {stopped:?}")),
    }
}

async fn prepare_schema(url: &str) -> Result<(), String> {
    let pool = open_url(url).await.map_err(|e| e.to_string())?;
    let mut migrations = product_migrations();
    let result = ensure_schema(&pool, &mut migrations).await;
    pool.close().await;
    result.map_err(|e| format!("migrate rejected: {e}"))
}
