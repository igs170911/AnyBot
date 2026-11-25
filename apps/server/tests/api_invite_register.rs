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

use anybot_app_test::http::RequestSpec;
use anybot_app_test::http::post;
use anybot_app_test::serve::serve_start;
use serde_json::json;

fn register_body(account: &str, password: &str, invite: &str, key: &str) -> RequestSpec {
    RequestSpec::new(
        &json!({"account": account, "password": password, "invite": invite}).to_string(),
    )
    .with_key(key)
}

#[tokio::test]
async fn t_server_invite_1() -> Result<(), String> {
    let served = serve_start("t_server_invite_1").await?;
    assert_eq!(served.started.account_count, 0);
    let invite = served
        .started
        .new_invite
        .clone()
        .clone()
        .ok_or("fresh boot must draw an invite".to_owned())?;
    let missing = post(
        &served.base_url(),
        "/v1/account/register",
        &register_body("newbie", "longenough", "", "k-missing"),
    )
    .await?;
    assert_eq!(missing.status, 403);
    assert_eq!(missing.code().as_deref(), Some("account-invite-invalid"));
    let wrong = post(
        &served.base_url(),
        "/v1/account/register",
        &register_body("newbie", "longenough", "zzzz-zzzz", "k-wrong"),
    )
    .await?;
    assert_eq!(wrong.status, 403);
    assert_eq!(wrong.code().as_deref(), Some("account-invite-invalid"));
    let ok = post(
        &served.base_url(),
        "/v1/account/register",
        &register_body("newbie", "longenough", &invite, "k-ok"),
    )
    .await?;
    assert_eq!(ok.status, 201);
    assert_eq!(
        ok.json.get("admin").and_then(serde_json::Value::as_bool),
        Some(true)
    );
    served.stop().await?;
    Ok(())
}

#[tokio::test]
async fn t_server_invite_4() -> Result<(), String> {
    let served = serve_start("t_server_invite_4").await?;
    let invite = served
        .started
        .new_invite
        .clone()
        .clone()
        .ok_or("fresh boot must draw an invite".to_owned())?;
    for n in 1..=5 {
        let ok = post(
            &served.base_url(),
            "/v1/account/register",
            &register_body(&format!("user{n}"), "longenough", &invite, &format!("k{n}")),
        )
        .await?;
        assert_eq!(ok.status, 201, "registration {n} must succeed");
    }
    let sixth = post(
        &served.base_url(),
        "/v1/account/register",
        &register_body("user6", "longenough", &invite, "k6"),
    )
    .await?;
    assert_eq!(sixth.status, 429);
    assert_eq!(sixth.code().as_deref(), Some("account-register-rate"));
    served.stop().await?;
    Ok(())
}

#[tokio::test]
async fn t_account_login_3() -> Result<(), String> {
    let served = serve_start("t_account_login_3").await?;
    let invite = served
        .started
        .new_invite
        .clone()
        .clone()
        .ok_or("fresh boot must draw an invite".to_owned())?;
    let short_name = post(
        &served.base_url(),
        "/v1/account/register",
        &register_body("a", "longenough", &invite, "k-a"),
    )
    .await?;
    assert_eq!(short_name.status, 400);
    assert_eq!(short_name.code().as_deref(), Some("account-name-rule"));
    let spaced = post(
        &served.base_url(),
        "/v1/account/register",
        &register_body("has space", "longenough", &invite, "k-space"),
    )
    .await?;
    assert_eq!(spaced.status, 400);
    assert_eq!(spaced.code().as_deref(), Some("account-name-rule"));
    let short_pass = post(
        &served.base_url(),
        "/v1/account/register",
        &register_body("validname", "7chars", &invite, "k-pass"),
    )
    .await?;
    assert_eq!(short_pass.status, 400);
    assert_eq!(short_pass.code().as_deref(), Some("account-password-rule"));
    served.stop().await?;
    Ok(())
}
