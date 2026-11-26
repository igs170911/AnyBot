use serde_json::json;

use anybot_app_server as _;
use anybot_app_test as _;
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

use anybot_app_test::http::{RequestSpec, get, post};
use anybot_app_test::{ServerProgram, db_start};
use test_harness::{Running, pg_stop};

use anybot_messages::{Catalog, Params};

const NEW_PASSWORD: &str = "sunny2233";
const OLD_PASSWORD: &str = "alicepassword";

fn server_bin() -> ServerProgram {
    ServerProgram::new(env!("CARGO_BIN_EXE_anybot-server"))
}

fn login_body(account: &str, password: &str, label: &str) -> String {
    json!({"account": account, "password": password, "label": label, "credential": "bearer"}).to_string()
}

fn login_spec(account: &str, password: &str, label: &str, key: &str) -> RequestSpec {
    RequestSpec::new(&login_body(account, password, label)).with_key(key)
}

fn register_body(account: &str, password: &str, invite: &str) -> String {
    json!({"account": account, "password": password, "invite": invite}).to_string()
}

fn password_body(old: &str, new: &str) -> String {
    json!({"old": old, "new": new}).to_string()
}

fn startup_params(version: &str, port: u16) -> Params {
    Params::new()
        .with_text("version", version)
        .with_text("addresses", &format!("127.0.0.1:{port}, [::1]:{port}"))
        .with_text("db_host", "127.0.0.1")
        .with_text("db_name", "anybot_test")
}

fn startup_line_of(db: &anybot_app_test::ServerDb) -> Result<String, String> {
    anybot_app_test::english(
        &mut Catalog::new(),
        anybot_messages::MessageCode::ServerStartupListening,
        &startup_params(
            anybot_foundation::log_version(&anybot_foundation::version_string()),
            db.listen_port,
        ),
    )
}

fn invite_from_line(line: &str) -> Result<String, String> {
    let tail = line
        .split("Generated invite code ")
        .nth(1)
        .ok_or("startup line carries no invite code".to_owned())?;
    let code = tail
        .split(';')
        .next()
        .ok_or("invite code part has no end".to_owned())?;
    Ok(code.to_owned())
}

#[tokio::test]
async fn password_change_rejects_a_short_new_password() -> Result<(), String> {
    let db = db_start("account_n10_1").await?;
    let outcome = run_n10_1(&db).await;
    let stopped = pg_stop(&db.pg).await;
    match outcome {
        Ok(_done) => stopped,
        Err(msg) => match stopped {
            Ok(_clean) => Err(msg),
            Err(stop_msg) => Err(format!("{msg}; {stop_msg}")),
        },
    }
}

async fn run_n10_1(db: &anybot_app_test::ServerDb) -> Result<(), String> {
    let running = start_server(db)?;
    let base = format!("http://127.0.0.1:{}", db.listen_port);
    drive_sessions(&base, running).await
}

fn start_server(db: &anybot_app_test::ServerDb) -> Result<Running, String> {
    let program = server_bin().program(db.args());
    let mut running = Running::start(&program, &db.dir).map_err(|e| e.clone())?;
    let announced = running.next_stdout_line()?;
    let expected = startup_line_of(db)?;
    if announced.trim() != expected {
        return Err(format!("startup line {announced} differs from {expected}"));
    }
    Ok(running)
}

async fn drive_sessions(base: &str, mut running: Running) -> Result<(), String> {
    let _no_accounts = running.next_stdout_line()?;
    let invite = invite_from_line(&running.next_stdout_line()?)?;
    let created = post(
        base,
        "/v1/account/register",
        &RequestSpec::new(&register_body("alice", OLD_PASSWORD, &invite)).with_key("k-reg"),
    )
    .await?;
    assert_eq!(created.status, 201, "register: {0}", created.json);
    let phones = open_three(base).await?;
    drop(created);
    assert_three_alive(base, &phones).await?;
    assert_ordered_errors(base, &phones.2).await?;
    assert_closed(base, &phones.0, &phones.1, &phones.2).await?;
    let result = running.kill();
    match result {
        Ok(_outcome) => Ok(()),
        Err(msg) => Err(msg),
    }
}

async fn open_three(base: &str) -> Result<(String, String, String), String> {
    let mut keys = Vec::new();
    for (index, label) in ["phoneP", "phoneP2", "computerA"].into_iter().enumerate() {
        let answer = post(
            base,
            "/v1/account/login",
            &login_spec("alice", OLD_PASSWORD, label, &format!("k-open{index}")),
        )
        .await?;
        assert_eq!(answer.status, 200, "login {label}: {0}", answer.json);
        keys.push(answer.bearer()?);
    }
    let phone = keys.first().ok_or("no first session".to_owned())?.clone();
    let phone2 = keys.get(1).ok_or("no second session".to_owned())?.clone();
    let computer = keys.get(2).ok_or("no third session".to_owned())?.clone();
    Ok((phone, phone2, computer))
}

async fn assert_three_alive(base: &str, phones: &(String, String, String)) -> Result<(), String> {
    let bearers = [&phones.0, &phones.1, &phones.2];
    for (index, bearer) in bearers.into_iter().enumerate() {
        let probe = get(
            base,
            "/v1/admin/invite",
            &RequestSpec::new("").with_bearer(bearer),
        )
        .await?;
        if probe.status != 200 {
            return Err(format!(
                "session {index} dead: {} {}",
                probe.status, probe.json
            ));
        }
        drop(probe);
    }
    Ok(())
}

async fn assert_ordered_errors(base: &str, bearer: &str) -> Result<(), String> {
    let short = post(
        base,
        "/v1/account/password",
        &RequestSpec::new(&password_body(OLD_PASSWORD, "abc"))
            .with_bearer(bearer)
            .with_key("k-short"),
    )
    .await?;
    assert_eq!(short.status, 400, "short: {0}", short.json);
    assert_eq!(short.code().as_deref(), Some("account-password-new-short"));
    let old_wrong = post(
        base,
        "/v1/account/password",
        &RequestSpec::new(&password_body("wrongold", "abcdefgh"))
            .with_bearer(bearer)
            .with_key("k-old"),
    )
    .await?;
    assert_eq!(old_wrong.status, 401, "old wrong: {0}", old_wrong.json);
    assert_eq!(
        old_wrong.code().as_deref(),
        Some("account-password-old-wrong"),
    );
    let changed = post(
        base,
        "/v1/account/password",
        &RequestSpec::new(&password_body(OLD_PASSWORD, NEW_PASSWORD))
            .with_bearer(bearer)
            .with_key("k-good"),
    )
    .await?;
    assert_eq!(changed.status, 200, "change: {0}", changed.json);
    let twin = post(
        base,
        "/v1/account/password",
        &RequestSpec::new(&password_body(OLD_PASSWORD, NEW_PASSWORD))
            .with_bearer(bearer)
            .with_key("k-good"),
    )
    .await?;
    assert_eq!(twin.status, 200, "replay: {0}", twin.json);
    assert_eq!(
        twin.json,
        json!({"ok": true}),
        "replay body: {0}",
        twin.json
    );
    Ok(())
}

async fn assert_closed(
    base: &str,
    phone: &str,
    phone2: &str,
    computer: &str,
) -> Result<(), String> {
    let probe = [phone, phone2];
    for (index, bearer) in probe.into_iter().enumerate() {
        let answer = post(
            base,
            "/v1/account/logout-all",
            &RequestSpec::new("{\"ok\":true}")
                .with_bearer(bearer)
                .with_key(&format!("k-probe{index}")),
        )
        .await?;
        assert_eq!(answer.status, 401, "phone probe: {0}", answer.json);
        assert_eq!(
            answer.code().as_deref(),
            Some("account-login-credentials"),
            "phone probe: {0}",
            answer.json
        );
    }
    let alive = post(
        base,
        "/v1/account/logout-all",
        &RequestSpec::new("{\"ok\":true}")
            .with_bearer(computer)
            .with_key("k-alive"),
    )
    .await?;
    assert_eq!(alive.status, 200, "computer probe: {0}", alive.json);
    Ok(())
}
