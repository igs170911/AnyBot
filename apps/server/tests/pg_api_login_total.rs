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
use std as _;
use test_harness as _;
use tokio as _;
use tracing as _;

use anybot_app_test::http::{FORWARDED_HEADER, RequestSpec, ResponseSpec, post};
use anybot_app_test::serve::serve_start_real;

fn login_spec(account: &str, password: &str, key: &str) -> RequestSpec {
    RequestSpec::new(
        &json!({"account": account, "password": password, "label": "test-device"}).to_string(),
    )
    .with_key(key)
}

fn register_body(account: &str, password: &str, invite: &str) -> String {
    json!({"account": account, "password": password, "invite": invite}).to_string()
}

fn invite_of(started: &anybot_server::Started) -> Result<String, String> {
    started
        .new_invite
        .clone()
        .ok_or("fresh boot must draw an invite".to_owned())
}

async fn register(
    base: &str,
    account: &str,
    password: &str,
    invite: &str,
    key: &str,
) -> Result<u16, String> {
    let answer = post(
        base,
        "/v1/account/register",
        &RequestSpec::new(&register_body(account, password, invite)).with_key(key),
    )
    .await?;
    Ok(answer.status)
}

#[tokio::test]
async fn t_account_login_9() -> Result<(), String> {
    let mut served = serve_start_real("t_account_login_9").await?;
    assert_disabled_answers(&mut served).await?;
    assert_equal_cost(&mut served).await?;
    served.stop().await?;
    Ok(())
}

async fn assert_disabled_answers(
    served: &mut anybot_app_test::serve::ServedReal,
) -> Result<(), String> {
    let invite = invite_of(&served.started)?;
    register(
        &served.base_url(),
        "root",
        "rootpassword",
        &invite,
        "k-root",
    )
    .await?;
    register(
        &served.base_url(),
        "alice",
        "alicepassword",
        &invite,
        "k-alice-reg",
    )
    .await?;
    let session = post(
        &served.base_url(),
        "/v1/account/login",
        &login_spec("root", "rootpassword", "k-session"),
    )
    .await?;
    let bearer = session.bearer()?;
    let disabled = post(
        &served.base_url(),
        "/v1/admin/accounts/alice/disable",
        &RequestSpec::new("{\"ok\":true}")
            .with_bearer(&bearer)
            .with_key("k-disable"),
    )
    .await?;
    assert_eq!(disabled.status, 200, "disable alice: {0}", disabled.json);
    let wrong = post(
        &served.base_url(),
        "/v1/account/login",
        &login_spec("alice", "wrongpassword", "k-wrong"),
    )
    .await?;
    assert_eq!(wrong.status, 401);
    assert_eq!(wrong.code().as_deref(), Some("account-login-credentials"));
    let right = post(
        &served.base_url(),
        "/v1/account/login",
        &login_spec("alice", "alicepassword", "k-right"),
    )
    .await?;
    assert_eq!(right.status, 403);
    assert_eq!(right.code().as_deref(), Some("account-login-disabled"));
    Ok(())
}

async fn assert_equal_cost(served: &mut anybot_app_test::serve::ServedReal) -> Result<(), String> {
    served
        .set_config_and_restart(anybot_server::KEY_TRUST_PROXY, "127.0.0.1")
        .await?;
    let base: std::sync::Arc<str> = served.base_url().into();
    let (ghost_times, alice_times) = timed_sides(&base).await?;
    let ghost_mid = median_ms(&ghost_times)?;
    let alice_mid = median_ms(&alice_times)?;
    let gap = ghost_mid.abs_diff(alice_mid);
    assert!(
        gap < 100,
        "ghost alice median gap {gap} ms ghost={ghost_mid} alice={alice_mid}"
    );
    Ok(())
}

fn ghost_passwords() -> Vec<String> {
    [
        "password",
        "12345678",
        "qwerty",
        "11111111",
        "123456789",
        "password1",
        "letmein",
        "0123456789",
        "iloveyou",
        "1234567",
        "admin",
        "welcome",
        "monkey",
        "abc12345",
        "football",
        "1234567890",
        "dragon",
        "trustno1",
        "sunshine",
        "princess",
        "master",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

async fn timed_sides(base: &std::sync::Arc<str>) -> Result<(Vec<u128>, Vec<u128>), String> {
    let mut ghost_times = Vec::new();
    let mut alice_times = Vec::new();
    for (round, password) in ghost_passwords().into_iter().enumerate() {
        let pair = run_pair(base, round, &password).await?;
        ghost_times.push(pair.0);
        alice_times.push(pair.1);
    }
    Ok((ghost_times, alice_times))
}

async fn run_pair(
    base: &std::sync::Arc<str>,
    round: usize,
    ghost_password: &str,
) -> Result<(u128, u128), String> {
    let ghost_key = format!("g{round}");
    let ghost_slot = format!("10.0.0.{round}");
    let alice_key = format!("a{round}");
    let alice_slot = format!("10.0.1.{round}");
    let ghost = side_post(base, "ghostname", ghost_password, &ghost_key, &ghost_slot);
    let alice = side_post(base, "alice", "wrongpassword", &alice_key, &alice_slot);
    let (ghost_answer, alice_answer) = tokio::join!(ghost, alice);
    Ok((
        elapsed_of(ghost_answer, "ghost")?,
        elapsed_of(alice_answer, "alice")?,
    ))
}

async fn side_post(
    base: &std::sync::Arc<str>,
    account: &str,
    password: &str,
    key: &str,
    slot: &str,
) -> Result<ResponseSpec, String> {
    let u = std::sync::Arc::clone(base);
    side_worker(u, account, password, key, slot).await
}

async fn side_worker(
    base: std::sync::Arc<str>,
    account: &str,
    password: &str,
    key: &str,
    slot: &str,
) -> Result<ResponseSpec, String> {
    let answer = post(
        &base,
        "/v1/account/login",
        &login_spec(account, password, key).with_header(FORWARDED_HEADER, slot),
    )
    .await?;
    assert_eq!(answer.status, 401, "side status: {0}", answer.json);
    assert_eq!(answer.code().as_deref(), Some("account-login-credentials"));
    Ok(answer)
}

fn elapsed_of(answer: Result<ResponseSpec, String>, side: &str) -> Result<u128, String> {
    let done = answer.map_err(|e| format!("{side} side post failed: {e}"))?;
    Ok(done.elapsed.as_millis())
}

fn median_ms(values: &[u128]) -> Result<u128, String> {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let mid = sorted.len() / 2;
    sorted
        .get(mid)
        .copied()
        .ok_or(format!("no median at {mid}"))
}
