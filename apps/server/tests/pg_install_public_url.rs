use tokio as _;

use sqlx as _;

use anybot_app_server as _;
use anybot_foundation as _;
use anybot_messages::{Catalog, MessageCode, Params};
use anybot_server as _;
use axum as _;
use clap as _;
use serde_json as _;
use test_harness as _;
use tracing as _;

use anybot_messages as _;
use anybot_store_server as _;
use rpassword as _;

use anybot_app_test::http::{RequestSpec, post};
use anybot_app_test::n106::{login_spec, register};
mod pg_install_shared;

use anybot_app_test::{
    bootstrap_args, check_both, check_second, get_status, public_url_check, start_live, stop_live,
};
use pg_install_shared::{db_start, english, server_bin, startup_line_of};
use test_harness::Running;

#[test]
fn public_url_check_rejects_a_body_without_the_url() {
    let missing = public_url_check("{\"public_url\":null}");
    assert!(missing.is_err(), "{missing:?}");
}

use test_harness::{pg_stop, run_program, test_dir, write_file_with_mode};

#[tokio::test]
async fn t_server_n14_1() -> Result<(), String> {
    let db = db_start("server_n14_1").await?;
    let mut first = start_live(&server_bin(), &db, &startup_line_of(&db)?).await?;
    let invite = drawn_invite_of(&mut first)?;
    let base = format!("http://127.0.0.1:{}", db.listen_port);
    let _created = register(&base, "dana", "danapassword", &invite, "k-reg").await?;
    let session = post(
        &base,
        "/v1/account/login",
        &login_spec("dana", "danapassword", "k-login", "10.9.9.9"),
    )
    .await?;
    let bearer = session.bearer()?;
    let changed = post(
        &base,
        "/v1/admin/public-url",
        &RequestSpec::new("{\"public_url\":\"https://next.example/\"}")
            .with_bearer(&bearer)
            .with_key("k-url"),
    )
    .await?;
    assert_eq!(
        changed.status, 200,
        "admin public-url body {0}",
        changed.json
    );
    stop_live(first).await.map(|_running| ())?;
    let second = start_live(&server_bin(), &db, &startup_line_of(&db)?).await?;
    let body = get_status(&db).await;
    let result = stop_live(second).await.map(|_running| ());
    let stopped = pg_stop(&db.pg).await;
    match check_both(body, result, stopped) {
        Ok(()) => Ok(()),
        Err(msg) => Err(msg),
    }
}

fn drawn_invite_of(running: &mut Running) -> Result<String, String> {
    running.next_stdout_line()?;
    let line = running.next_stdout_line()?;
    let code = line
        .strip_prefix("Generated invite code ")
        .and_then(|rest| rest.split(';').next())
        .ok_or(format!("invite line missing: {line}"))?;
    Ok(code.to_owned())
}

#[tokio::test]
async fn t_server_n14_4() -> Result<(), String> {
    let db = db_start("server_n14_4").await?;
    let mut program = server_bin().program(db.args());
    program = program.with_env("ANYBOT_RELAY_PUBLIC_URL", "https://env.example/");
    program = program.with_env("ANYBOT_RELAY_INVITE", "zzzz-zzzz");
    let mut running = Running::start(&program, &db.dir)?;
    let announced = running.next_stdout_line()?;
    let expected = startup_line_of(&db)?;
    let body = get_status(&db).await;
    let stored = stored_config(&db.url).await;
    let _result = running.kill();
    let stopped = pg_stop(&db.pg).await;
    match env_check(&announced, &expected, body, stored, stopped) {
        Ok(()) => Ok(()),
        Err(msg) => Err(msg),
    }
}

async fn stored_config(url: &str) -> Result<anybot_server::ServerConfig, String> {
    let pool = anybot_server::open_url(url)
        .await
        .map_err(|e| e.to_string())?;
    let rows = anybot_server::read_config_rows(&pool)
        .await
        .map_err(|e| e.to_string())?;
    pool.close().await;
    anybot_server::ServerConfig::from_rows(rows)
}

fn env_check(
    announced: &str,
    expected: &str,
    body: Result<String, String>,
    stored: Result<anybot_server::ServerConfig, String>,
    stopped: Result<(), String>,
) -> Result<(), String> {
    if announced.trim() != expected {
        return Err(format!("startup line {announced} differs from {expected}"));
    }
    let config = stored?;
    let code_ref = &config.invite_code;
    let drawn = match code_ref {
        Some(code) => code.expose().to_owned(),
        None => return Err("the stored invite code must be the drawn one".to_owned()),
    };
    if drawn == "zzzz-zzzz" || config.public_url.as_deref() == Some("https://env.example/") {
        return Err(format!(
            "environment variables changed the stored settings: {config:?}"
        ));
    }
    if drawn.len() < 8 {
        return Err("the stored invite code must be the drawn one".to_owned());
    }
    match body {
        Ok(text) => {
            if text.contains("env.example") || text.contains("zzzz") {
                return Err(format!("environment leaked into status: {text}"));
            }
            stopped
        }
        Err(msg) => Err(msg),
    }
}
#[tokio::test]
async fn t_server_n14_2() -> Result<(), String> {
    let dir = test_dir("server_n14_2")?;
    let path = dir.join("boot.conf");
    write_file_with_mode(&path, "postgresql://x:y@127.0.0.1:5432/z\n", 0o644)?;
    let outcome = run_program(&server_bin().program(bootstrap_args(&path)), &dir)?;
    if outcome.code == 0 {
        return Err("world-readable bootstrap was accepted".to_owned());
    }
    let expected = english(
        &mut Catalog::new(),
        MessageCode::ServerBootstrapPermissionsTooOpen,
        &Params::new()
            .with_text("mode", "644")
            .with_text("path", &path.display().to_string()),
    )?;
    if !outcome.stderr.contains(&expected) {
        return Err(format!("{outcome:?} misses {expected}"));
    }
    Ok(())
}

#[tokio::test]
async fn public_url_server_reserves_the_port_for_the_first() -> Result<(), String> {
    let db = db_start("public_url_port_taken").await?;
    let first = start_live(&server_bin(), &db, &startup_line_of(&db)?).await?;
    let second = run_program(&server_bin().program(db.args()), &db.dir)?;
    check_second(&second, db.listen_port)?;
    stop_live(first).await.map(|_running| ())?;
    pg_stop(&db.pg).await
}
