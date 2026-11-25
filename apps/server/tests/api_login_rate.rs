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

use anybot_app_test::http::{FORWARDED_HEADER, RequestSpec, ResponseSpec, post};
use anybot_app_test::serve::{serve_start, serve_start_real};

fn login_body(account: &str, password: &str) -> String {
    json!({"account": account, "password": password, "label": "test-device"}).to_string()
}

fn login_spec(account: &str, password: &str, key: &str) -> RequestSpec {
    RequestSpec::new(&login_body(account, password)).with_key(key)
}

fn register_body(account: &str, password: &str, invite: &str) -> String {
    json!({"account": account, "password": password, "invite": invite}).to_string()
}

async fn register(
    base: &str,
    account: &str,
    password: &str,
    invite: &str,
    key: &str,
) -> Result<ResponseSpec, String> {
    let answer = post(
        base,
        "/v1/account/register",
        &RequestSpec::new(&register_body(account, password, invite)).with_key(key),
    )
    .await?;
    if answer.status != 201 {
        return Err(format!(
            "register {account} status {} body {}",
            answer.status, answer.json
        ));
    }
    Ok(answer)
}

fn invite_of(started: &anybot_server::Started) -> Result<String, String> {
    started
        .new_invite
        .clone()
        .ok_or("fresh boot must draw an invite".to_owned())
}

async fn make_admin(served: &anybot_app_test::serve::Served) -> Result<String, String> {
    let invite = invite_of(&served.started)?;
    register(
        &served.base_url(),
        "alice",
        "alicepassword",
        &invite,
        "k-admin",
    )
    .await?;
    Ok(invite)
}

async fn login_step(
    served: &anybot_app_test::serve::Served,
    account: &str,
    password: &str,
    key: &str,
) -> Result<ResponseSpec, String> {
    login_step_with(served, account, password, key, None).await
}

async fn login_step_with(
    served: &anybot_app_test::serve::Served,
    account: &str,
    password: &str,
    key: &str,
    forwarded: Option<&str>,
) -> Result<ResponseSpec, String> {
    let mut walker = anybot_app_test::ClockWalker::start(&served.clock);
    let mut spec = login_spec(account, password, key);
    if let Some(address) = forwarded {
        spec = spec.with_header(FORWARDED_HEADER, address);
    }
    let outcome = post(&served.base_url(), "/v1/account/login", &spec).await?;
    walker.stop().await?;
    Ok(outcome)
}

#[tokio::test]
async fn t_safety_n10_1() -> Result<(), String> {
    let served = serve_start("t_safety_n10_1").await?;
    let invite = make_admin(&served).await?;
    register(&served.base_url(), "bob", "bobpassword", &invite, "k-bob").await?;
    for round in 1..=8 {
        let fail = login_step(&served, "bob", "wrongpassword", &format!("f{round}")).await?;
        assert_eq!(fail.status, 401, "failure {round}");
    }
    let ninth = login_step(&served, "bob", "bobpassword", "k-ninth").await?;
    assert_eq!(ninth.status, 429);
    assert_eq!(ninth.code().as_deref(), Some("account-login-rate"));
    let _advanced = served.clock.advance(std::time::Duration::from_secs(300));
    let after = login_step(&served, "bob", "bobpassword", "k-after").await?;
    assert_eq!(after.status, 200);
    served.stop().await?;
    Ok(())
}

#[tokio::test]
async fn t_safety_n10_5() -> Result<(), String> {
    let mut served = serve_start("t_safety_n10_5").await?;
    let invite = make_admin(&served).await?;
    register(&served.base_url(), "bob", "bobpassword", &invite, "k-bob").await?;
    for round in 1..=8 {
        let fail = login_step(&served, "bob", "wrongpassword", &format!("f{round}")).await?;
        assert_eq!(fail.status, 401, "failure {round}");
    }
    served.stop_server().await?;
    served.restart_server().await?;
    let after_restart = login_step(&served, "bob", "bobpassword", "k-after-restart").await?;
    assert_eq!(after_restart.status, 429);
    served.stop().await?;
    Ok(())
}

#[tokio::test]
async fn t_server_n2_5() -> Result<(), String> {
    let mut served = serve_start("t_server_n2_5").await?;
    let invite = make_admin(&served).await?;
    register(&served.base_url(), "bob", "bobpassword", &invite, "k-bob").await?;
    for round in 1..=7 {
        let fail = login_step(&served, "bob", "wrongpassword", &format!("f{round}")).await?;
        assert_eq!(fail.status, 401, "failure {round}");
    }
    served.stop_server().await?;
    served.restart_server().await?;
    let eighth = login_step(&served, "bob", "wrongpassword", "f8").await?;
    assert_eq!(eighth.status, 401);
    let ninth = login_step(&served, "bob", "bobpassword", "k-ninth").await?;
    assert_eq!(ninth.status, 429);
    served.stop().await?;
    Ok(())
}

#[tokio::test]
async fn t_account_login_4() -> Result<(), String> {
    let served = serve_start_real("t_account_login_4").await?;
    let invite = invite_of(&served.started)?;
    register(
        &served.base_url(),
        "alice",
        "alicepassword",
        &invite,
        "k-admin",
    )
    .await?;
    let pair = run_pairs(&served).await?;
    let alice_mid = median(&pair.0)?;
    let ghost_mid = median(&pair.1)?;
    let gap = alice_mid.abs_diff(ghost_mid);
    assert!(gap < 50, "median gap {gap} ms");
    served.stop().await?;
    Ok(())
}

async fn run_pairs(
    served: &anybot_app_test::serve::ServedReal,
) -> Result<(Vec<u128>, Vec<u128>), String> {
    let mut alice_times = Vec::new();
    let mut ghost_times = Vec::new();
    for round in 1..=6 {
        let fail = post(
            &served.base_url(),
            "/v1/account/login",
            &login_spec("alice", "wrongpassword", &format!("a{round}")),
        )
        .await?;
        assert_eq!(fail.status, 401);
        assert!(fail.elapsed.as_millis() >= 500, "alice attempt too fast");
        alice_times.push(fail.elapsed.as_millis());
        let ghost = post(
            &served.base_url(),
            "/v1/account/login",
            &login_spec("ghostname", "anypassword", &format!("g{round}")),
        )
        .await?;
        assert_eq!(ghost.status, 401);
        assert_eq!(ghost.code().as_deref(), Some("account-login-credentials"));
        assert!(ghost.elapsed.as_millis() >= 500, "ghost attempt too fast");
        ghost_times.push(ghost.elapsed.as_millis());
    }
    Ok((alice_times, ghost_times))
}

fn median(values: &[u128]) -> Result<u128, String> {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let mid = sorted.len() / 2;
    sorted
        .get(mid)
        .copied()
        .ok_or(format!("no median at {mid}"))
}
