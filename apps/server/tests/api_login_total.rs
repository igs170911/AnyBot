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

async fn wave(base: &str, slots: usize, rounds: usize) -> Result<Vec<ResponseSpec>, String> {
    let mut tasks = Vec::new();
    for slot in 0..slots {
        let address = std::sync::Arc::new(format!("{slot}.{slot}.{slot}.{slot}"));
        let url = std::sync::Arc::new(base.to_owned());
        for round in 0..rounds {
            let key = format!("c{slot}-{round}");
            let addr = std::sync::Arc::clone(&address);
            let u = std::sync::Arc::clone(&url);
            let handle = tokio::spawn(async move {
                post(
                    u.as_str(),
                    "/v1/account/login",
                    &login_spec("bob", "wrongpassword", &key)
                        .with_header(FORWARDED_HEADER, addr.as_str()),
                )
                .await
            });
            tasks.push(handle);
        }
    }
    let mut all: Vec<ResponseSpec> = Vec::new();
    for handle in tasks {
        let joined = handle
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| format!("wave post failed: {e}"))?;
        all.push(joined);
    }
    Ok(all)
}

async fn tails(base: &str, slots: usize, rounds: usize) -> Result<Vec<ResponseSpec>, String> {
    let mut extra = Vec::new();
    for slot in 0..slots {
        let address = format!("{slot}.{slot}.{slot}.{slot}");
        let last = post(
            base,
            "/v1/account/login",
            &login_spec("bob", "wrongpassword", &format!("c{slot}-{rounds}"))
                .with_header(FORWARDED_HEADER, &address),
        )
        .await?;
        extra.push(last);
    }
    Ok(extra)
}

async fn parity_side(base: &str, account: &str, key_prefix: &str) -> Result<Vec<u128>, String> {
    let mut times = Vec::new();
    for round in 0..3 {
        let fail = post(
            base,
            "/v1/account/login",
            &login_spec(account, "wrongpassword", &format!("{key_prefix}{round}")),
        )
        .await?;
        assert_eq!(fail.status, 401, "{account} parity: {0}", fail.json);
        times.push(fail.elapsed.as_millis());
    }
    Ok(times)
}

fn median(values: &mut [u128]) -> Result<u128, String> {
    values.sort_unstable();
    let mid = values.len() / 2;
    values
        .get(mid)
        .copied()
        .ok_or(format!("no median at {mid}"))
}

#[tokio::test]
async fn t_safety_n10_2() -> Result<(), String> {
    let mut served = serve_start_real("t_safety_n10_2").await?;
    let invite = invite_of(&served.started)?;
    assert_eq!(
        register(
            &served.base_url(),
            "alice",
            "alicepassword",
            &invite,
            "k-admin"
        )
        .await?,
        201
    );
    assert_eq!(
        register(&served.base_url(), "bob", "bobpassword", &invite, "k-bob").await?,
        201
    );
    served
        .set_config_and_restart(anybot_server::KEY_TRUST_PROXY, "127.0.0.1")
        .await?;
    let base = served.base_url();
    let first = wave(&base, 6, 6).await?;
    let last = tails(&base, 6, 6).await?;
    let mut all = first;
    all.extend(last);
    check_global_window(&all);
    parity_check(&base).await?;
    served.stop().await?;
    Ok(())
}

fn check_global_window(all: &[ResponseSpec]) {
    assert_eq!(all.len(), 42);
    let blocked = all.iter().filter(|r| r.status == 429).count();
    let accepted = all.iter().filter(|r| r.status == 401).count();
    assert_eq!(accepted, 40, "accepted failures: {accepted}");
    assert_eq!(blocked, 2, "blocked: {blocked}");
    for outcome in all {
        if outcome.status == 401 {
            assert!(
                outcome.elapsed.as_millis() >= 500,
                "slow fail took {} ms",
                outcome.elapsed.as_millis(),
            );
        }
    }
}

async fn parity_check(base: &str) -> Result<(), String> {
    let left = std::sync::Arc::new(base.to_owned());
    let right = std::sync::Arc::clone(&left);
    let a = left.as_str().to_owned();
    let b = right.as_str().to_owned();
    let alice_handle = tokio::spawn(async move { parity_side(&a, "alice", "m").await });
    let ghost_handle = tokio::spawn(async move { parity_side(&b, "ghost", "g").await });
    let alice_out = alice_handle.await.map_err(|e| e.to_string())??;
    let ghost_out = ghost_handle.await.map_err(|e| e.to_string())??;
    let alice_mid = median(&mut alice_out.clone())?;
    let ghost_mid = median(&mut ghost_out.clone())?;
    let gap = alice_mid.abs_diff(ghost_mid);
    assert!(gap < 50, "parity gap {gap} ms");
    Ok(())
}
