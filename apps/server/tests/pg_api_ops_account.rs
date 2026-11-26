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
use anybot_app_test::serve::serve_start;

fn login_spec(account: &str, password: &str, key: &str) -> RequestSpec {
    RequestSpec::new(
        &json!({"account": account, "password": password, "label": "test-device", "credential": "bearer"}).to_string(),
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

async fn logout_probe(base: &str, bearer: &str, key: &str) -> Result<u16, String> {
    let answer = post(
        base,
        "/v1/account/logout",
        &RequestSpec::new("{\"ok\":true}")
            .with_bearer(bearer)
            .with_key(key),
    )
    .await?;
    Ok(answer.status)
}

#[tokio::test]
async fn t_account_n10_2() -> Result<(), String> {
    let served = serve_start("t_account_n10_2").await?;
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
    let bearer = bearer_of_fake(&served, "alice", "alicepassword", "k-login").await?;
    for round in 1..=8 {
        let fail = password_step(&served, &bearer, "wrongold", &format!("p{round}")).await?;
        assert_eq!(fail.status, 401, "failure {round}: {0}", fail.json);
        assert_eq!(fail.code().as_deref(), Some("account-password-old-wrong"));
    }
    let ninth = password_step(&served, &bearer, "wrongold", "p9").await?;
    assert_eq!(ninth.status, 429, "ninth: {0}", ninth.json);
    assert_eq!(ninth.code().as_deref(), Some("account-password-rate"));
    served.stop().await?;
    Ok(())
}

#[tokio::test]
async fn t_account_n9_3() -> Result<(), String> {
    let served = serve_start("t_account_n9_3").await?;
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
    let first = post(
        &served.base_url(),
        "/v1/account/login",
        &login_spec("bob", "bobpassword", "k-first").with_header(FORWARDED_HEADER, "x"),
    )
    .await?;
    assert_eq!(first.status, 200);
    let p_key = first.bearer()?;
    expiry_check(&served, &p_key).await?;
    let keys = fill_devices(&served, p_key).await?;
    eviction_check(&served, &keys).await?;
    served.stop().await?;
    Ok(())
}

async fn expiry_check(served: &anybot_app_test::serve::Served, p_key: &str) -> Result<(), String> {
    let _advanced = served
        .clock
        .advance(std::time::Duration::from_mins(90 * 24 * 60 + 1));
    let idle_probe = logout_probe(&served.base_url(), p_key, "k-probe").await?;
    assert_eq!(idle_probe, 401, "P after 90 days");
    Ok(())
}

async fn fill_devices(
    served: &anybot_app_test::serve::Served,
    first_key: String,
) -> Result<Vec<String>, String> {
    let mut keys = vec![first_key];
    for round in 2..=52 {
        let login = login_step_noclock(served, "bob", &format!("d{round}")).await?;
        assert_eq!(login.status, 200, "device {round}: {0}", login.json);
        keys.push(login.bearer()?);
    }
    Ok(keys)
}

async fn eviction_check(
    served: &anybot_app_test::serve::Served,
    keys: &[String],
) -> Result<(), String> {
    let oldest = keys.get(1).ok_or("missing oldest key".to_owned())?.as_str();
    let survivor = keys
        .get(51)
        .ok_or("missing survivor key".to_owned())?
        .as_str();
    let evicted = logout_probe(&served.base_url(), oldest, "k-evict").await?;
    assert_eq!(evicted, 401, "oldest evicted");
    let alive = logout_probe(&served.base_url(), survivor, "k-survivor").await?;
    assert_eq!(alive, 200, "survivor");
    Ok(())
}

async fn login_step_noclock(
    served: &anybot_app_test::serve::Served,
    account: &str,
    key: &str,
) -> Result<ResponseSpec, String> {
    let _advanced = served.clock.advance(std::time::Duration::from_secs(1));
    post(
        &served.base_url(),
        "/v1/account/login",
        &login_spec(account, "bobpassword", key),
    )
    .await
}

async fn password_step(
    served: &anybot_app_test::serve::Served,
    bearer: &str,
    old: &str,
    key: &str,
) -> Result<ResponseSpec, String> {
    let mut walker = anybot_app_test::ClockWalker::start(&served.clock);
    let outcome = post(
        &served.base_url(),
        "/v1/account/password",
        &RequestSpec::new(&json!({"old": old, "new": "brandnewpassword"}).to_string())
            .with_bearer(bearer)
            .with_key(key),
    )
    .await?;
    walker.stop().await?;
    Ok(outcome)
}

async fn bearer_of_fake(
    served: &anybot_app_test::serve::Served,
    account: &str,
    password: &str,
    key: &str,
) -> Result<String, String> {
    let session = post(
        &served.base_url(),
        "/v1/account/login",
        &login_spec(account, password, key),
    )
    .await?;
    if session.status != 200 {
        return Err(format!("login {account} status {}", session.status));
    }
    session.bearer()
}
