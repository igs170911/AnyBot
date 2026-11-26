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

use std::time::Duration;

use anybot_app_test::http::{LOGIN_STEP, RequestSpec, ResponseSpec, post, post_driven};
use anybot_app_test::serve::serve_start;

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

async fn bearer_of(
    served: &anybot_app_test::serve::Served,
    account: &str,
    password: &str,
    key: &str,
) -> Result<String, String> {
    let session = post_driven(
        &served.clock,
        &served.base_url(),
        "/v1/account/login",
        &login_spec(account, password, key),
        LOGIN_STEP,
        Duration::from_secs(15),
    )
    .await?;
    if session.status != 200 {
        return Err(format!(
            "login {account} status {} body {}",
            session.status, session.json
        ));
    }
    session.bearer()
}

async fn admin_post(
    base: &str,
    path: &str,
    bearer: &str,
    body: &str,
    key: &str,
) -> Result<ResponseSpec, String> {
    post(
        base,
        path,
        &RequestSpec::new(body).with_bearer(bearer).with_key(key),
    )
    .await
}

#[tokio::test]
async fn t_conn_post_2() -> Result<(), String> {
    let served = serve_start("t_conn_post_2").await?;
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
    let bearer = bearer_of(&served, "alice", "alicepassword", "k-alice").await?;
    let first = url_call(&served, &bearer, "https://one.example.test", "k-same").await?;
    assert_eq!(first.status, 200, "first public-url: {0}", first.json);
    let replay = url_call(&served, &bearer, "https://one.example.test", "k-same").await?;
    assert_eq!(replay.status, 200);
    let conflict = url_call(&served, &bearer, "https://two.example.test", "k-same").await?;
    assert_eq!(conflict.status, 409, "conflict: {0}", conflict.json);
    assert_eq!(
        conflict.code().as_deref(),
        Some("account-idempotency-conflict")
    );
    served.stop().await?;
    Ok(())
}

async fn url_call(
    served: &anybot_app_test::serve::Served,
    bearer: &str,
    url: &str,
    key: &str,
) -> Result<ResponseSpec, String> {
    admin_post(
        &served.base_url(),
        "/v1/admin/public-url",
        bearer,
        &json!({"public_url": url}).to_string(),
        key,
    )
    .await
}

#[tokio::test]
async fn t_server_n13_4() -> Result<(), String> {
    let served = serve_start("t_server_n13_4").await?;
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
    let alice_bearer = bearer_of(&served, "alice", "alicepassword", "k-alice").await?;
    let bob_bearer = bearer_of(&served, "bob", "bobpassword", "k-bob-login").await?;
    self_disable_checks(&served, &alice_bearer).await?;
    bob_admin_checks(&served, &bob_bearer).await?;
    served.stop().await?;
    Ok(())
}

async fn self_disable_checks(
    served: &anybot_app_test::serve::Served,
    bearer: &str,
) -> Result<(), String> {
    let self_disable = admin_post(
        &served.base_url(),
        "/v1/admin/accounts/alice/disable",
        bearer,
        "{\"ok\":true}",
        "k-self",
    )
    .await?;
    assert_eq!(
        self_disable.status, 403,
        "self disable: {0}",
        self_disable.json
    );
    assert_eq!(
        self_disable.code().as_deref(),
        Some("account-admin-self-disable")
    );
    Ok(())
}

async fn bob_admin_checks(
    served: &anybot_app_test::serve::Served,
    bearer: &str,
) -> Result<(), String> {
    let bob_rotate = admin_post(
        &served.base_url(),
        "/v1/admin/invite/rotate",
        bearer,
        "{\"ok\":true}",
        "k-bob-rotate",
    )
    .await?;
    assert_eq!(bob_rotate.status, 403, "bob rotate: {0}", bob_rotate.json);
    assert_eq!(bob_rotate.code().as_deref(), Some("account-admin-only"));
    let bob_disable = admin_post(
        &served.base_url(),
        "/v1/admin/accounts/bob/disable",
        bearer,
        "{\"ok\":true}",
        "k-bob-disable",
    )
    .await?;
    assert_eq!(
        bob_disable.status, 403,
        "bob self disable: {0}",
        bob_disable.json
    );
    Ok(())
}

const SCAN_PASSWORDS: [&str; 5] = [
    "alicepassword",
    "bobpassword",
    "brandnew2233",
    "sunny2233",
    "hunter2password",
];

#[tokio::test]
async fn pg_idempotency_records_hold_no_secrets() -> Result<(), String> {
    let served = serve_start("idem_no_secrets").await?;
    let base = served.base_url();
    let invite = invite_of(&served.started)?;
    assert_eq!(
        register(&base, "alice", "alicepassword", &invite, "k-a").await?,
        201
    );
    assert_eq!(
        register(&base, "bob", "bobpassword", &invite, "k-b").await?,
        201
    );
    let alice_bearer = bearer_of(&served, "alice", "alicepassword", "k-alice").await?;
    let change = post(
        &base,
        "/v1/account/password",
        &RequestSpec::new(&json!({"old": "alicepassword", "new": "brandnew2233"}).to_string())
            .with_bearer(&alice_bearer)
            .with_key("k-change"),
    )
    .await?;
    assert_eq!(change.status, 200, "change body {0}", change.json);
    let reset = admin_post(
        &base,
        "/v1/admin/accounts/bob/reset-password",
        &alice_bearer,
        &json!({"new": "sunny2233"}).to_string(),
        "k-reset",
    )
    .await?;
    assert_eq!(reset.status, 200, "reset body {0}", reset.json);
    let rows = sqlx::query!("SELECT key, request_hash, status_code, body FROM idempotency")
        .fetch_all(&served.pool)
        .await
        .map_err(|e| e.to_string())?;
    assert!(
        !rows.is_empty(),
        "the flows above must record idempotency rows"
    );
    for row in rows {
        let line = format!("{}{}{}", row.key, row.request_hash, row.body);
        for secret in SCAN_PASSWORDS {
            assert!(
                !line.contains(secret),
                "record {} holds a password",
                row.key
            );
        }
        assert!(!line.contains("argon2"), "record {} holds a hash", row.key);
        assert!(
            !line.contains(&alice_bearer),
            "record {} holds a session key",
            row.key
        );
    }
    served.stop().await?;
    Ok(())
}
