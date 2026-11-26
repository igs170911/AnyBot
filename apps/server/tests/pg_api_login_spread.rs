use anybot_server_test as _;
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
use anybot_app_test::serve::serve_start;

use anybot_server::KEY_TRUST_PROXY;

const N2_1_ACCOUNT_LIMIT: usize = 40;
const N2_1_BLOCKED_AT: usize = N2_1_ACCOUNT_LIMIT.saturating_add(1);

fn login_body(account: &str, password: &str) -> String {
    json!({"account": account, "password": password, "label": "test-device", "credential": "bearer"}).to_string()
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

async fn login_step_xff(
    served: &anybot_app_test::serve::Served,
    account: &str,
    password: &str,
    key: &str,
    forwarded: &str,
) -> Result<ResponseSpec, String> {
    login_step_with(served, account, password, key, Some(forwarded)).await
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
async fn t_server_n2_1() -> Result<(), String> {
    let mut served = serve_start("t_server_n2_1").await?;
    let invite = make_admin(&served).await?;
    register(&served.base_url(), "bob", "bobpassword", &invite, "k-bob").await?;
    served
        .set_config_and_restart(KEY_TRUST_PROXY, "127.0.0.1")
        .await?;
    for round in 1..=8 {
        let fail = login_step_xff(
            &served,
            "bob",
            "wrongpassword",
            &format!("a{round}"),
            "1.1.1.1",
        )
        .await?;
        assert_eq!(fail.status, 401, "pair failure {round}");
    }
    let ninth = login_step_xff(&served, "bob", "bobpassword", "k-ninth", "1.1.1.1").await?;
    assert_eq!(ninth.status, 429);
    spread_failures(&served).await?;
    all_addresses_blocked(&served).await?;
    served.stop().await?;
    Ok(())
}

async fn spread_failures(served: &anybot_app_test::serve::Served) -> Result<(), String> {
    let plans = [
        ("2.2.2.2", 6),
        ("3.3.3.3", 6),
        ("4.4.4.4", 6),
        ("5.5.5.5", 5),
        ("6.6.6.6", 5),
        ("7.7.7.7", 5),
    ];
    let mut round_total: usize = 8;
    for (address, count) in plans {
        for _ in 0..count {
            round_total = round_total
                .checked_add(1)
                .ok_or("round overflow".to_owned())?;
            let expected = expected_at(round_total);
            let fail = login_step_xff(
                served,
                "bob",
                "wrongpassword",
                &format!("b{round_total}"),
                address,
            )
            .await?;
            assert_eq!(
                fail.status, expected,
                "spread attempt {round_total} from {address} body {}",
                fail.json
            );
        }
    }
    Ok(())
}

fn expected_at(attempt: usize) -> u16 {
    if attempt >= N2_1_BLOCKED_AT { 429 } else { 401 }
}

async fn all_addresses_blocked(served: &anybot_app_test::serve::Served) -> Result<(), String> {
    let addresses = [
        "1.1.1.1", "2.2.2.2", "3.3.3.3", "4.4.4.4", "5.5.5.5", "6.6.6.6", "7.7.7.7",
    ];
    for (number, address) in addresses.into_iter().enumerate() {
        let blocked =
            login_step_xff(served, "bob", "bobpassword", &format!("c{number}"), address).await?;
        assert_eq!(
            blocked.status, 429,
            "address {address} body {}",
            blocked.json
        );
    }
    let direct = login_step(served, "bob", "bobpassword", "k-direct-blocked").await?;
    assert_eq!(direct.status, 429, "direct body {}", direct.json);
    Ok(())
}
