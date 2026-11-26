use anybot_server_test as _;
use serde_json::json;

use anybot_app_server as _;
use anybot_app_test as _;
use anybot_app_test::{ServerProgram, db_start};
use anybot_foundation as _;
use anybot_messages as _;
use anybot_server as _;
use anybot_store_server as _;
use axum as _;
use clap as _;
use rpassword as _;
use serde_json as _;
use sqlx as _;
use test_harness as _;
use tokio as _;
use tracing as _;

use anybot_app_test::http::{RequestSpec, ResponseSpec, post};
use test_harness::launch::run_program_stdin;
use test_harness::{Program, Running, http_get_addr, sleep_real};

fn server_bin() -> ServerProgram {
    ServerProgram::new(env!("CARGO_BIN_EXE_anybot-server"))
}

fn register_body(account: &str, password: &str, invite: &str) -> String {
    json!({"account": account, "password": password, "invite": invite}).to_string()
}

fn login_body(account: &str, password: &str) -> String {
    json!({"account": account, "password": password, "label": "test-device", "credential": "bearer"}).to_string()
}

fn invite_code_from_line(line: &str) -> Result<String, String> {
    let parts: Vec<&str> = line.split("Generated invite code ").collect();
    if parts.len() != 2 {
        return Err(format!("invite line {line} has no code"));
    }
    let code = parts
        .get(1)
        .ok_or("no invite code part")?
        .split(';')
        .next()
        .unwrap_or_default()
        .to_owned();
    Ok(code)
}

async fn health_ok(port: u16) -> Result<(), String> {
    for _attempt in 0..40 {
        match http_get_addr(&format!("127.0.0.1:{port}"), "/v1/health").await {
            Ok(_out) => return Ok(()),
            Err(_e) => sleep_real(std::time::Duration::from_millis(250)).await,
        }
    }
    Err(format!("no answer from {port}"))
}

fn run_reset_cli(
    bin: &str,
    bootstrap: &str,
    account: &str,
    password: &str,
    workdir: &std::path::Path,
) -> Result<i32, String> {
    let program = Program::new(
        bin,
        vec![
            "--bootstrap".to_owned(),
            bootstrap.to_owned(),
            "reset-password".to_owned(),
            account.to_owned(),
        ],
    )
    .with_timeout(std::time::Duration::from_secs(15));
    let outcome = run_program_stdin(&program, workdir, &format!("{password}\n{password}\n"))?;
    Ok(outcome.code)
}

async fn login(
    base: &str,
    account: &str,
    password: &str,
    key: &str,
) -> Result<ResponseSpec, String> {
    post(
        base,
        "/v1/account/login",
        &RequestSpec::new(&login_body(account, password)).with_key(key),
    )
    .await
}

async fn login_status(base: &str, account: &str, password: &str, key: &str) -> Result<u16, String> {
    let answer = login(base, account, password, key).await?;
    Ok(answer.status)
}

#[tokio::test]
async fn t_account_n11_2() -> Result<(), String> {
    let db = db_start("account_n11_2").await?;
    let program = server_bin()
        .program(db.args())
        .with_timeout(std::time::Duration::from_secs(40));
    let mut running = Running::start(&program, &db.dir).map_err(|e| e.clone())?;
    let announced = running.next_stdout_line().map_err(|e| e.clone())?;
    assert!(announced.contains("listening on"), "line: {announced}");
    let _no_accounts = running.next_stdout_line().map_err(|e| e.clone())?;
    let invite_line = running.next_stdout_line().map_err(|e| e.clone())?;
    let invite = invite_code_from_line(&invite_line)?;
    let base = format!("http://127.0.0.1:{}", db.listen_port);
    let ok = post(
        &base,
        "/v1/account/register",
        &RequestSpec::new(&register_body("alice", "alicepassword", &invite)).with_key("k-admin"),
    )
    .await?;
    assert_eq!(ok.status, 201, "admin register: {0}", ok.json);
    let session = login(&base, "alice", "alicepassword", "k-login").await?;
    assert_eq!(session.status, 200, "alice login: {0}", session.json);
    let bearer = session.bearer()?;
    reset_and_check(&base, &db.bootstrap, &bearer).await?;
    health_ok(db.listen_port).await?;
    let _killed = running.kill().map_err(|e| e.clone())?;
    db.stop().await?;
    Ok(())
}

async fn reset_and_check(
    base: &str,
    bootstrap: &std::path::Path,
    bearer: &str,
) -> Result<(), String> {
    let code = run_reset_cli(
        env!("CARGO_BIN_EXE_anybot-server"),
        &bootstrap.display().to_string(),
        "alice",
        "brandnewpassword",
        &std::path::PathBuf::from("."),
    )?;
    assert_eq!(code, 0, "reset cli exit code");
    assert_eq!(
        login_status(base, "alice", "alicepassword", "k-old").await?,
        401,
        "old password"
    );
    assert_eq!(
        login_status(base, "alice", "brandnewpassword", "k-new").await?,
        200,
        "new password"
    );
    let old_session = post(
        base,
        "/v1/account/logout",
        &RequestSpec::new("{\"ok\":true}")
            .with_bearer(bearer)
            .with_key("k-probe"),
    )
    .await?;
    assert_eq!(
        old_session.status, 401,
        "old session: {0}",
        old_session.json
    );
    Ok(())
}

#[tokio::test]
async fn reset_password_normalizes_the_account_and_counts_characters() -> Result<(), String> {
    let db = db_start("reset_password_chars").await?;
    let program = server_bin()
        .program(db.args())
        .with_timeout(std::time::Duration::from_secs(40));
    let mut running = Running::start(&program, &db.dir).map_err(|e| e.clone())?;
    let announced = running.next_stdout_line().map_err(|e| e.clone())?;
    assert!(announced.contains("listening on"), "line: {announced}");
    let _no_accounts = running.next_stdout_line().map_err(|e| e.clone())?;
    let invite_line = running.next_stdout_line().map_err(|e| e.clone())?;
    let invite = invite_code_from_line(&invite_line)?;
    let base = format!("http://127.0.0.1:{}", db.listen_port);
    let ok = post(
        &base,
        "/v1/account/register",
        &RequestSpec::new(&register_body("alice", "alicepassword", &invite)).with_key("k-admin"),
    )
    .await?;
    assert_eq!(ok.status, 201, "admin register: {0}", ok.json);
    reset_normalizes_and_counts_chars(&base, &db.bootstrap).await?;
    health_ok(db.listen_port).await?;
    let _killed = running.kill().map_err(|e| e.clone())?;
    db.stop().await?;
    Ok(())
}

async fn reset_normalizes_and_counts_chars(
    base: &str,
    bootstrap: &std::path::Path,
) -> Result<(), String> {
    let mixed = run_reset_cli(
        env!("CARGO_BIN_EXE_anybot-server"),
        &bootstrap.display().to_string(),
        "Alice",
        "mixedcasepassword",
        &std::path::PathBuf::from("."),
    )?;
    assert_eq!(mixed, 0, "mixed-case account must normalize");
    assert_eq!(
        login_status(base, "alice", "mixedcasepassword", "k-mixed").await?,
        200,
        "normalized reset"
    );
    let short_chars = run_reset_cli(
        env!("CARGO_BIN_EXE_anybot-server"),
        &bootstrap.display().to_string(),
        "alice",
        "\u{5BC6}\u{78BC}\u{4E03}\u{500B}\u{5B57}\u{52A0}\u{4E00}",
        &std::path::PathBuf::from("."),
    )?;
    assert_ne!(short_chars, 0, "seven characters must be rejected");
    let restored = run_reset_cli(
        env!("CARGO_BIN_EXE_anybot-server"),
        &bootstrap.display().to_string(),
        "alice",
        "brandnewpassword",
        &std::path::PathBuf::from("."),
    )?;
    assert_eq!(restored, 0, "restore password after the short reject");
    Ok(())
}
