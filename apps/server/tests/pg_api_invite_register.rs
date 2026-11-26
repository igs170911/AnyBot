use anybot_app_server as _;
use anybot_app_test as _;
use anybot_foundation as _;
use anybot_messages as _;
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

use anybot_app_test::http::{FORWARDED_HEADER, RequestSpec, ResponseSpec, post};
use anybot_app_test::serve::serve_start;
use serde_json::json;

fn register_body(account: &str, password: &str, invite: &str, key: &str) -> RequestSpec {
    RequestSpec::new(
        &json!({"account": account, "password": password, "invite": invite}).to_string(),
    )
    .with_key(key)
    .with_header(FORWARDED_HEADER, "10.9.9.9")
}

#[tokio::test]
async fn fresh_boot_rejects_a_missing_or_wrong_invite_and_accepts_the_first_admin()
-> Result<(), String> {
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

const REGISTER_DEADLINE_MS: u128 = 2000;

#[tokio::test]
async fn sequential_registration_answers_within_two_seconds() -> Result<(), String> {
    let served = serve_start("seq_register_2s").await?;
    let invite = served
        .started
        .new_invite
        .clone()
        .ok_or("fresh boot must draw an invite".to_owned())?;
    for (number, account) in ["firstone", "secondone"].into_iter().enumerate() {
        let answer = post(
            &served.base_url(),
            "/v1/account/register",
            &register_body(account, "longenough", &invite, &format!("k-seq{number}")),
        )
        .await?;
        assert_eq!(answer.status, 201, "registration {number} must succeed");
        assert!(
            answer.elapsed.as_millis() < REGISTER_DEADLINE_MS,
            "registration {number} took {} ms",
            answer.elapsed.as_millis()
        );
    }
    served.stop().await?;
    Ok(())
}

#[tokio::test]
async fn concurrent_registration_creates_only_one_admin() -> Result<(), String> {
    let served = serve_start("conc_register_admin").await?;
    let invite = served
        .started
        .new_invite
        .clone()
        .ok_or("fresh boot must draw an invite".to_owned())?;
    let base = served.base_url();
    let alpha = tokio::spawn(register_task(
        base.clone(),
        "alphaone".to_owned(),
        invite.clone(),
        "k-alpha".to_owned(),
    ));
    let beta = tokio::spawn(register_task(
        base,
        "betatwo".to_owned(),
        invite,
        "k-beta".to_owned(),
    ));
    let (alpha_answer, alpha_ms) = alpha.await.map_err(|e| e.to_string())??;
    let (beta_answer, beta_ms) = beta.await.map_err(|e| e.to_string())??;
    assert_eq!(
        alpha_answer.status, 201,
        "alpha status: {0}",
        alpha_answer.json
    );
    assert_eq!(
        beta_answer.status, 201,
        "beta status: {0}",
        beta_answer.json
    );
    assert!(
        alpha_ms < REGISTER_DEADLINE_MS && beta_ms < REGISTER_DEADLINE_MS,
        "alpha {alpha_ms} ms, beta {beta_ms} ms"
    );
    let alpha_admin = alpha_answer
        .json
        .get("admin")
        .and_then(serde_json::Value::as_bool);
    let beta_admin = beta_answer
        .json
        .get("admin")
        .and_then(serde_json::Value::as_bool);
    assert_eq!(
        alpha_admin,
        beta_admin.map(|admin| !admin),
        "exactly one registration must be the first admin: {} {}",
        alpha_answer.json,
        beta_answer.json
    );
    served.stop().await?;
    Ok(())
}

async fn register_task(
    base: String,
    account: String,
    invite: String,
    key: String,
) -> Result<(ResponseSpec, u128), String> {
    let answer = post(
        &base,
        "/v1/account/register",
        &register_body(&account, "longenough", &invite, &key),
    )
    .await?;
    let millis = answer.elapsed.as_millis();
    Ok((answer, millis))
}

#[tokio::test]
async fn register_rejects_a_one_character_name_a_space_and_a_short_password() -> Result<(), String>
{
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
        &register_body("validname", "1234567", &invite, "k-pass"),
    )
    .await?;
    assert_eq!(short_pass.status, 400);
    assert_eq!(short_pass.code().as_deref(), Some("account-password-rule"));
    served.stop().await?;
    Ok(())
}

#[tokio::test]
async fn register_stores_the_normalized_account_name() -> Result<(), String> {
    let served = serve_start("t_account_login_1").await?;
    let invite = served
        .started
        .new_invite
        .clone()
        .clone()
        .ok_or("fresh boot must draw an invite".to_owned())?;
    let base = served.base_url();
    let registered = post(
        &base,
        "/v1/account/register",
        &register_body("Bob", "longenough", &invite, "k-bob-reg"),
    )
    .await?;
    assert_eq!(registered.status, 201);
    let stored = anybot_store_server::accounts::find_account(&served.pool, "bob")
        .await
        .map_err(|e| e.to_string())?;
    assert!(stored.is_some(), "register must store the normalized name");
    let logged = post(
        &base,
        "/v1/account/login",
        &RequestSpec::new(
            &json!({"account": "Bob", "password": "longenough", "label": "d1", "credential": "bearer"}).to_string(),
        )
        .with_key("k-bob-login"),
    )
    .await?;
    assert_eq!(logged.status, 200, "body: {}", logged.json);
    served.stop().await?;
    Ok(())
}
