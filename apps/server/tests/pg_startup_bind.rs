use anybot_app_server as _;
use anybot_app_test as _;
use anybot_app_test::{ServerDb, ServerProgram, db_start, english};
use anybot_foundation as _;
use anybot_messages::{Catalog, MessageCode, Params};
use anybot_server as _;
use axum as _;
use clap as _;
use serde_json as _;
use sqlx as _;
use test_harness as _;
use tracing as _;

use test_harness::{Running, http_get_addr, pg_stop, sleep_real};

fn server_bin() -> ServerProgram {
    ServerProgram::new(env!("CARGO_BIN_EXE_anybot-server"))
}

#[tokio::test]
async fn startup_logs_announce_no_accounts_and_a_new_invite() -> Result<(), String> {
    let db = db_start("startup_logs").await?;
    let program = server_bin()
        .program(db.args())
        .with_timeout(std::time::Duration::from_secs(40));
    let mut running = Running::start(&program, &db.dir)?;
    let _announced = running.next_stdout_line()?;
    let no_accounts = running.next_stdout_line()?;
    let invite = running.next_stdout_line()?;
    let result = running.kill();
    let _stopped = pg_stop(&db.pg).await;
    check_startup_logs(&no_accounts, &invite, result)
}

fn check_startup_logs(
    no_accounts: &str,
    invite: &str,
    result: Result<test_harness::Outcome, String>,
) -> Result<(), String> {
    let want = english(
        &mut Catalog::new(),
        MessageCode::ServerStartupNoAccounts,
        &Params::new(),
    )?;
    if no_accounts.trim() != want {
        return Err(format!(
            "no-accounts line {no_accounts} differs from {want}"
        ));
    }
    let code = invite_code_from_line(invite)?;
    if invite_code_shape(&code).is_err() {
        return Err(format!("bad generated invite {code}"));
    }
    match result {
        Ok(_outcome) => Ok(()),
        Err(msg) => Err(msg),
    }
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

fn invite_code_shape(code: &str) -> Result<(), String> {
    let parts: Vec<&str> = code.split('-').collect();
    let head = parts.first().ok_or("no invite head")?;
    let tail = parts.get(1).ok_or("no invite tail")?;
    if parts.len() != 2 || head.len() != 4 || tail.len() != 4 {
        return Err(format!("bad invite shape {code}"));
    }
    for ch in code.chars() {
        if ch == '-' {
            continue;
        }
        if !ch.is_ascii_alphanumeric() || "01lOI".contains(ch) {
            return Err(format!("bad invite character {ch} in {code}"));
        }
    }
    Ok(())
}

#[tokio::test]
async fn t_account_bind_1() -> Result<(), String> {
    let db = db_start("account_bind_1").await?;
    let mut running = start_and_hold(&db)?;
    let v4 = health_twice(db.listen_port, "127.0.0.1").await;
    let v6 = health_twice(db.listen_port, "[::1]").await;
    let third = http_get_addr(&format!("127.0.0.2:{}", db.listen_port), "/v1/health")
        .await
        .is_ok();
    let result = running.kill();
    let _stopped = pg_stop(&db.pg).await;
    match health_shapes(v4, v6, third, result) {
        Ok(()) => Ok(()),
        Err(msg) => Err(msg),
    }
}

fn start_and_hold(db: &ServerDb) -> Result<Running, String> {
    let mut running = Running::start(&server_bin().program(db.args()), &db.dir)?;
    let announced = running.next_stdout_line()?;
    if !announced.contains("listening on") {
        let _result = running.kill();
        return Err(format!("startup line {announced} is not a listening line"));
    }
    Ok(running)
}

async fn health_twice(port: u16, host: &str) -> Result<(u16, String), String> {
    for _attempt in 0..40 {
        match http_get_addr(&format!("{host}:{port}"), "/v1/health").await {
            Ok(out) => return Ok(out),
            Err(_e) => sleep_real(std::time::Duration::from_millis(250)).await,
        }
    }
    Err(format!("no answer from {host}:{port}"))
}

fn health_shapes(
    v4: Result<(u16, String), String>,
    v6: Result<(u16, String), String>,
    third_ok: bool,
    result: Result<test_harness::Outcome, String>,
) -> Result<(), String> {
    match v4 {
        Ok((200, _)) => {}
        other => return Err(format!("loopback v4 health {other:?}")),
    }
    match v6 {
        Ok((200, _)) => {}
        other => return Err(format!("loopback v6 health {other:?}")),
    }
    if third_ok {
        return Err("a non-listening loopback address answered".to_owned());
    }
    match result {
        Ok(_outcome) => Ok(()),
        Err(msg) => Err(msg),
    }
}
