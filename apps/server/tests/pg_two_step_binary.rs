use anybot_app_server as _;
use anybot_domain as _;
use anybot_server as _;
use anybot_server_test as _;
use anybot_store_server as _;
use axum as _;
use clap as _;
use rpassword as _;
use serde_json as _;
use sqlx as _;
use test_harness as _;
use tokio as _;
use tracing as _;

use std::time::Duration;

use anybot_app_test::n106::register;
use anybot_app_test::two_step_client::{
    TwoStepUser, challenge_of, enable_two_step_at, login_password, step_two,
};
use anybot_app_test::{
    ServerDb, ServerProgram, bootstrap_text, comparable_dump, db_start, english, open_pool,
    wait_health,
};
use anybot_foundation::{Clock, MasterKey, SystemClock};
use anybot_messages::{Catalog, MessageCode, Params};
use test_harness::scratch::write_file_with_mode;
use test_harness::{Program, Running, run_program};

const SERVER_RUN_BUDGET: Duration = Duration::from_secs(40);

fn server_bin() -> ServerProgram {
    ServerProgram::new(env!("CARGO_BIN_EXE_anybot-server"))
}

fn real_now_ms() -> u64 {
    SystemClock.now_ms().millis()
}

fn base_of(db: &ServerDb) -> String {
    format!("http://127.0.0.1:{}", db.listen_port)
}

fn invite_code_from_line(line: &str) -> Result<String, String> {
    let tail = line
        .split("Generated invite code ")
        .nth(1)
        .ok_or(format!("invite line {line} has no code"))?;
    let code = tail.split(';').next().ok_or("empty invite code")?;
    Ok(code.trim().to_owned())
}

async fn start_server(db: &ServerDb) -> Result<Running, String> {
    let program = server_bin()
        .program(db.args())
        .with_timeout(SERVER_RUN_BUDGET);
    let mut running = Running::start(&program, &db.dir)?;
    let announced = running.next_stdout_line()?;
    assert!(announced.contains("listening on"), "line: {announced}");
    wait_health(db).await?;
    Ok(running)
}

async fn first_start(db: &ServerDb) -> Result<(Running, String), String> {
    let program = server_bin()
        .program(db.args())
        .with_timeout(SERVER_RUN_BUDGET);
    let mut running = Running::start(&program, &db.dir)?;
    let announced = running.next_stdout_line()?;
    assert!(announced.contains("listening on"), "line: {announced}");
    let _no_accounts = running.next_stdout_line()?;
    let invite = invite_code_from_line(&running.next_stdout_line()?)?;
    wait_health(db).await?;
    Ok((running, invite))
}

async fn enabled_user(db: &ServerDb, account: &str, invite: &str) -> Result<TwoStepUser, String> {
    let password = format!("{account}password");
    register(
        &base_of(db),
        account,
        &password,
        invite,
        &format!("k-{account}"),
    )
    .await?;
    enable_two_step_at(&base_of(db), account, &password, real_now_ms).await
}

async fn login_with_code(db: &ServerDb, user: &TwoStepUser, key: &str) -> Result<u16, String> {
    let base = base_of(db);
    let challenge = challenge_of(&base, user, &format!("{key}-password")).await?;
    let code = user.authenticator.code_at(real_now_ms())?;
    let answer = step_two(&base, &challenge, &code, &format!("{key}-code")).await?;
    Ok(answer.status)
}

fn wrong_key_start(db: &ServerDb) -> Result<(), String> {
    let wrong = MasterKey::generate().map_err(|e| e.to_string())?;
    write_file_with_mode(&db.bootstrap, &bootstrap_text(&db.url, &wrong), 0o600)?;
    let outcome = run_program(&server_bin().program(db.args()), &db.dir)?;
    assert_ne!(
        outcome.code, 0,
        "wrong master key must not start: {outcome:?}"
    );
    let want = english(
        &mut Catalog::new(),
        MessageCode::ServerMasterKeyMismatch,
        &Params::new(),
    )?;
    assert!(
        outcome.stdout.contains(&want),
        "output names the wrong key: {outcome:?}"
    );
    write_file_with_mode(
        &db.bootstrap,
        &bootstrap_text(&db.url, &db.master_key),
        0o600,
    )
}

async fn privacy_6_body(db: &ServerDb) -> Result<(), String> {
    let (mut running, invite) = first_start(db).await?;
    let alice = enabled_user(db, "alice", &invite).await?;
    let _killed = running.kill()?;
    let before = comparable_dump(db)?;
    wrong_key_start(db)?;
    let after = comparable_dump(db)?;
    assert!(
        before == after,
        "the database changed during the wrong-key start"
    );
    let mut running = start_server(db).await?;
    let status = login_with_code(db, &alice, "back").await?;
    let _killed = running.kill()?;
    assert_eq!(
        status, 200,
        "alice logs in with a code after the right key is back"
    );
    Ok(())
}

#[tokio::test]
async fn t_server_privacy_6() -> Result<(), String> {
    let db = db_start("t_server_privacy_6").await?;
    let outcome = privacy_6_body(&db).await;
    let stopped = db.stop().await;
    outcome.and(stopped)
}

fn reset_two_step(db: &ServerDb, account: &str) -> Result<test_harness::Outcome, String> {
    let mut args = db.args();
    args.extend(["reset-two-step".to_owned(), account.to_owned()]);
    let program = Program::new(env!("CARGO_BIN_EXE_anybot-server"), args)
        .with_timeout(Duration::from_secs(15));
    run_program(&program, &db.dir)
}

async fn two_step_rows(db: &ServerDb, account: &str) -> Result<(i64, i64), String> {
    let pool = open_pool(db).await?;
    let secrets: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM two_step WHERE account = $1")
        .bind(account)
        .fetch_one(&pool)
        .await
        .map_err(|e| e.to_string())?;
    let codes: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM two_step_recovery WHERE account = $1")
            .bind(account)
            .fetch_one(&pool)
            .await
            .map_err(|e| e.to_string())?;
    pool.close().await;
    Ok((secrets, codes))
}

async fn reset_alice_while_running(db: &ServerDb, running: &mut Running) -> Result<(), String> {
    let outcome = reset_two_step(db, "alice")?;
    assert_eq!(outcome.code, 0, "reset-two-step: {outcome:?}");
    let done = english(
        &mut Catalog::new(),
        MessageCode::ServerTwoStepResetDone,
        &Params::new().with_text("account", "alice"),
    )?;
    assert!(outcome.stdout.contains(&done), "reset output: {outcome:?}");
    let exited = running.child().try_wait().map_err(|e| e.to_string())?;
    assert!(exited.is_none(), "server stopped during the reset");
    wait_health(db).await
}

fn unknown_account_is_refused(db: &ServerDb) -> Result<(), String> {
    let unknown = reset_two_step(db, "nobody")?;
    assert_ne!(unknown.code, 0, "unknown account: {unknown:?}");
    assert!(
        unknown.stderr.contains("account not found"),
        "unknown account: {unknown:?}"
    );
    Ok(())
}

async fn n3_6_body(db: &ServerDb, running: &mut Running, invite: &str) -> Result<(), String> {
    let alice = enabled_user(db, "alice", invite).await?;
    let bob = enabled_user(db, "bob", invite).await?;
    assert_eq!(two_step_rows(db, "alice").await?, (1, 8));
    reset_alice_while_running(db, running).await?;
    let plain = login_password(&base_of(db), "alice", &alice.password, "alice-plain").await?;
    assert_eq!(
        plain.status, 200,
        "alice with only the password: {}",
        plain.json
    );
    assert_eq!(two_step_rows(db, "alice").await?, (0, 0));
    let bob_login = login_password(&base_of(db), "bob", &bob.password, "bob-plain").await?;
    assert_eq!(
        bob_login.status, 202,
        "bob still needs the second step: {}",
        bob_login.json
    );
    unknown_account_is_refused(db)
}

#[tokio::test]
async fn t_account_n3_6() -> Result<(), String> {
    let db = db_start("t_account_n3_6").await?;
    let outcome = match first_start(&db).await {
        Ok((mut running, invite)) => {
            let body = n3_6_body(&db, &mut running, &invite).await;
            let _killed = running.kill();
            body
        }
        Err(err) => Err(err),
    };
    let stopped = db.stop().await;
    outcome.and(stopped)
}
