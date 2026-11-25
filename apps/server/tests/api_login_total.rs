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
    served
        .set_config_and_restart(anybot_server::KEY_TRUST_PROXY, "127.0.0.1")
        .await?;
    let base = std::sync::Arc::new(served.base_url());
    let (ghost_times, alice_times) = timed_sides(&base).await?;
    let ghost_mid = median_ms(&ghost_times)?;
    let alice_mid = median_ms(&alice_times)?;
    let gap = ghost_mid.abs_diff(alice_mid);
    assert!(gap < 100, "ghost alice median gap {gap} ms ghost={ghost_mid} alice={alice_mid}");
    served.stop().await?;
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
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

async fn timed_sides(
    base: &std::sync::Arc<String>,
) -> Result<(Vec<u128>, Vec<u128>), String> {
    let mut plans: Vec<(String, String, String, String)> = Vec::new();
    for (round, password) in ghost_passwords().into_iter().enumerate().take(19) {
        plans.push((
            "ghostname".to_owned(),
            password,
            format!("g{round}"),
            format!("10.0.0.{round}"),
        ));
    }
    for round in 0..19 {
        plans.push((
            "alice".to_owned(),
            "wrongpassword".to_owned(),
            format!("a{round}"),
            format!("10.0.1.{round}"),
        ));
    }
    let mut tasks = Vec::new();
    for (account, password, key, slot) in plans {
        let u = std::sync::Arc::clone(base);
        let handle = tokio::spawn(async move {
            post(
                u.as_str(),
                "/v1/account/login",
                &login_spec(&account, &password, &key).with_header(FORWARDED_HEADER, &slot),
            )
            .await
        });
        tasks.push(handle);
    }
    let mut ghost_times = Vec::new();
    let mut alice_times = Vec::new();
    for (index, handle) in tasks.into_iter().enumerate() {
        let answer = handle
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| format!("side post failed: {e}"))?;
        assert_eq!(answer.status, 401, "side status: {0}", answer.json);
        assert_eq!(answer.code().as_deref(), Some("account-login-credentials"));
        let even = index % 2 == 0;
        if even {
            ghost_times.push(answer.elapsed.as_millis());
        } else {
            alice_times.push(answer.elapsed.as_millis());
        }
    }
    Ok((ghost_times, alice_times))
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
