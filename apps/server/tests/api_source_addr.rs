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

use std::time::Duration;

use anybot_app_test::http::{FORWARDED_HEADER, LOGIN_STEP, RequestSpec, post_driven};
use anybot_app_test::serve::{boot_pair, serve_start};

fn login_spec(account: &str, password: &str, key: &str) -> RequestSpec {
    RequestSpec::new(
        &json!({"account": account, "password": password, "label": "test-device"}).to_string(),
    )
    .with_key(key)
}

async fn fail_login(
    served: &anybot_app_test::serve::Served,
    account: &str,
    key: &str,
    forwarded: Option<&str>,
    v6: bool,
) -> Result<(), String> {
    let mut spec = login_spec(account, "wrongpassword", key);
    if let Some(value) = forwarded {
        spec = spec.with_header(FORWARDED_HEADER, value);
    }
    let base = if v6 {
        served.base_url_v6()
    } else {
        served.base_url()
    };
    let fail = post_driven(
        &served.clock,
        &base,
        "/v1/account/login",
        &spec,
        LOGIN_STEP,
        Duration::from_secs(15),
    )
    .await?;
    assert_eq!(fail.status, 401, "failure {key}");
    Ok(())
}

async fn login_blocked(
    served: &anybot_app_test::serve::Served,
    account: &str,
    key: &str,
    forwarded: &str,
    v6: bool,
) -> Result<u16, String> {
    let spec = login_spec(account, &format!("{account}password"), key)
        .with_header(FORWARDED_HEADER, forwarded);
    let base = if v6 {
        served.base_url_v6()
    } else {
        served.base_url()
    };
    let answer = post_driven(
        &served.clock,
        &base,
        "/v1/account/login",
        &spec,
        LOGIN_STEP,
        Duration::from_secs(15),
    )
    .await?;
    Ok(answer.status)
}

async fn set_trust(served: &mut anybot_app_test::serve::Served, value: &str) -> Result<(), String> {
    served
        .set_config_and_restart(anybot_server::KEY_TRUST_PROXY, value)
        .await
}

const PROXY_SECRET: &str = "x-anybot-proxy-secret";
const SECRET_VALUE: &str = "testproxysecret";

async fn set_secret(
    served: &mut anybot_app_test::serve::Served,
    value: &str,
) -> Result<(), String> {
    served
        .set_config_and_restart(anybot_server::KEY_PROXY_SECRET, value)
        .await
}

#[tokio::test]
async fn t_server_n1_1() -> Result<(), String> {
    let served = serve_start("t_server_n1_1").await?;
    boot_pair(&served).await?;
    for round in 1..=8 {
        fail_login(&served, "bob", &format!("x{round}"), Some("1.2.3.4"), false).await?;
    }
    let blocked = login_blocked(&served, "bob", "k-blocked", "5.6.7.8", false).await?;
    assert_eq!(blocked, 429);
    served.stop().await?;
    Ok(())
}

#[tokio::test]
async fn t_server_n1_2() -> Result<(), String> {
    let mut served = serve_start("t_server_n1_2").await?;
    boot_pair(&served).await?;
    set_trust(&mut served, "127.0.0.1").await?;
    for round in 1..=8 {
        fail_login(
            &served,
            "bob",
            &format!("p{round}"),
            Some("6.6.6.6, 1.2.3.4"),
            false,
        )
        .await?;
    }
    let blocked_forwarded =
        login_blocked(&served, "bob", "k-blocked", "6.6.6.6, 1.2.3.4", false).await?;
    assert_eq!(blocked_forwarded, 429);
    for round in 1..=8 {
        fail_login(
            &served,
            "bob",
            &format!("v{round}"),
            Some("6.6.6.6, 1.2.3.4"),
            true,
        )
        .await?;
    }
    let blocked_v6 =
        login_blocked(&served, "bob", "k-blocked-v6", "6.6.6.6, 1.2.3.4", true).await?;
    assert_eq!(blocked_v6, 429);
    let fresh_address = login_blocked(&served, "alice", "k-fresh", "9.9.9.9", false).await?;
    assert_eq!(fresh_address, 200);
    served.stop().await?;
    Ok(())
}

async fn login_with_secret(
    served: &anybot_app_test::serve::Served,
    key: &str,
    forwarded: &str,
) -> Result<u16, String> {
    let spec = login_spec("bob", "bobpassword", key)
        .with_header(FORWARDED_HEADER, forwarded)
        .with_header(PROXY_SECRET, SECRET_VALUE);
    let answer = post_driven(
        &served.clock,
        &served.base_url(),
        "/v1/account/login",
        &spec,
        LOGIN_STEP,
        Duration::from_secs(15),
    )
    .await?;
    Ok(answer.status)
}

#[tokio::test]
async fn t_server_n1_3() -> Result<(), String> {
    let mut served = serve_start("t_server_n1_3").await?;
    boot_pair(&served).await?;
    set_trust(&mut served, "127.0.0.1").await?;
    set_secret(&mut served, SECRET_VALUE).await?;
    for round in 1..=8 {
        fail_login(&served, "bob", &format!("n{round}"), Some("1.2.3.4"), false).await?;
    }
    let blocked = login_blocked(&served, "bob", "k-blocked", "1.2.3.4", false).await?;
    assert_eq!(blocked, 429);
    let passed = login_with_secret(&served, "k-secret-pass", "8.8.8.8").await?;
    assert_eq!(passed, 200, "secret header must unlock the forwarded value");
    served.stop().await?;
    Ok(())
}

#[tokio::test]
async fn t_safety_n11_1() -> Result<(), String> {
    let served = serve_start("t_safety_n11_1").await?;
    boot_pair(&served).await?;
    for round in 1..=8 {
        let head = if round <= 4 { "1.2.3.4" } else { "5.6.7.8" };
        fail_login(&served, "bob", &format!("h{round}"), Some(head), false).await?;
    }
    let blocked = login_blocked(&served, "bob", "k-blocked", "9.9.9.9", false).await?;
    assert_eq!(blocked, 429);
    served.stop().await?;
    Ok(())
}

#[tokio::test]
async fn t_safety_n11_2() -> Result<(), String> {
    let mut served = serve_start("t_safety_n11_2").await?;
    boot_pair(&served).await?;
    set_trust(&mut served, "127.0.0.1").await?;
    for round in 1..=8 {
        fail_login(
            &served,
            "bob",
            &format!("f{round}"),
            Some("9.9.9.9, 1.2.3.4"),
            false,
        )
        .await?;
    }
    let blocked_last =
        login_blocked(&served, "bob", "k-blocked-last", "9.9.9.9, 1.2.3.4", false).await?;
    assert_eq!(blocked_last, 429);
    let _cleared = served.clock.advance(Duration::from_secs(301));
    let long_value = "a".repeat(65);
    for round in 1..=8 {
        fail_login(
            &served,
            "bob",
            &format!("l{round}"),
            Some(&long_value),
            false,
        )
        .await?;
    }
    let blocked_long = login_blocked(&served, "bob", "k-blocked-long", &long_value, false).await?;
    assert_eq!(blocked_long, 429);
    let fresh_long = login_blocked(
        &served,
        "alice",
        "k-fresh-long",
        "b".repeat(65).as_str(),
        false,
    )
    .await?;
    assert_eq!(
        fresh_long, 200,
        "a 65-char forwarded value must count at the peer"
    );
    served.stop().await?;
    Ok(())
}

#[tokio::test]
async fn t_safety_n11_3() -> Result<(), String> {
    let mut served = serve_start("t_safety_n11_3").await?;
    boot_pair(&served).await?;
    set_trust(&mut served, "127.0.0.1").await?;
    set_secret(&mut served, SECRET_VALUE).await?;
    for round in 1..=8 {
        fail_login(&served, "bob", &format!("c{round}"), Some("1.2.3.4"), true).await?;
    }
    let blocked_v6 = login_blocked(&served, "bob", "k-blocked-v6", "1.2.3.4", true).await?;
    assert_eq!(blocked_v6, 429);
    let passed = login_with_secret(&served, "k-secret-pass", "7.7.7.7").await?;
    assert_eq!(passed, 200, "secret header must unlock the forwarded value");
    served.stop().await?;
    Ok(())
}
