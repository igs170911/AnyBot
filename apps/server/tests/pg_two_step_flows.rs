use anybot_app_server as _;
use anybot_domain as _;
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

use serde_json::json;

use anybot_app_test::http::{RequestSpec, post};
use anybot_app_test::n106::{invite_of, register};
use anybot_app_test::serve::{Served, serve_start};
use anybot_app_test::two_step_client::{
    TWO_STEP_LOGIN_PATH, TwoStepUser, challenge_of, enable_two_step, fake_now_ms, login_password,
    signed_post, step_two, step_two_spec, step_two_walked, wrong_code_for,
};

async fn served_with(test: &str, accounts: &[&str]) -> Result<(Served, Vec<TwoStepUser>), String> {
    let served = serve_start(test).await?;
    let invite = invite_of(&served.started)?;
    let mut users = Vec::new();
    for account in accounts {
        let password = format!("{account}password");
        register(
            &served.base_url(),
            account,
            &password,
            &invite,
            &format!("k-{account}"),
        )
        .await?;
        users.push(enable_two_step(&served.base_url(), &served.clock, account, &password).await?);
    }
    Ok((served, users))
}

fn cookie_login_spec(user: &TwoStepUser, origin: &str) -> RequestSpec {
    let body = json!({"account": user.account, "password": user.password, "label": "desk", "credential": "cookie"});
    RequestSpec::new(&body.to_string())
        .with_key("cookie-login")
        .with_header("Origin", origin)
}

#[tokio::test]
async fn second_step_issues_the_credential_the_first_step_asked_for() -> Result<(), String> {
    let (served, users) = served_with("two_step_cookie", &["alice"]).await?;
    let user = users.first().ok_or("no user")?;
    let base = served.base_url();
    let first = post(&base, "/v1/account/login", &cookie_login_spec(user, &base)).await?;
    assert_eq!(first.status, 202, "first step: {}", first.json);
    assert!(
        first.header_values("set-cookie").is_empty(),
        "no cookie before the code"
    );
    let challenge = first.field_str("challenge")?;
    let code = user.authenticator.code_at(fake_now_ms(&served.clock))?;
    let native = post(
        &base,
        TWO_STEP_LOGIN_PATH,
        &step_two_spec(&challenge, &code, "native"),
    )
    .await?;
    assert_eq!(
        native.status, 403,
        "cookie step without an origin: {}",
        native.json
    );
    let spec = step_two_spec(&challenge, &code, "browser").with_header("Origin", &base);
    let second = post(&base, TWO_STEP_LOGIN_PATH, &spec).await?;
    assert_eq!(second.status, 200, "second step: {}", second.json);
    assert_eq!(
        second.header_values("set-cookie").len(),
        1,
        "one session cookie"
    );
    assert!(
        second.json.get("bearer_key").is_none(),
        "cookie login has no bearer key"
    );
    let bearer = challenge_of(&base, user, "bearer-first").await?;
    let previous = user
        .authenticator
        .previous_code_at(fake_now_ms(&served.clock))?;
    let bearer_step = step_two(&base, &bearer, &previous, "bearer-second").await?;
    assert_eq!(
        bearer_step.status, 200,
        "bearer second step: {}",
        bearer_step.json
    );
    assert!(bearer_step.header_values("set-cookie").is_empty());
    assert!(bearer_step.bearer().is_ok(), "bearer login has a key");
    served.stop().await
}

#[tokio::test]
async fn second_step_replay_opens_a_new_session_or_repeats_the_failure() -> Result<(), String> {
    let (served, users) = served_with("two_step_replay", &["alice"]).await?;
    let user = users.first().ok_or("no user")?;
    let base = served.base_url();
    let challenge = challenge_of(&base, user, "replay-login").await?;
    let wrong = wrong_code_for(&user.authenticator, fake_now_ms(&served.clock))?;
    let failed = step_two_walked(&base, &served.clock, &challenge, &wrong, "replay-wrong").await?;
    assert_eq!(failed.status, 401);
    let again = step_two(&base, &challenge, &wrong, "replay-wrong").await?;
    assert_eq!(again.status, 401, "replayed failure: {}", again.json);
    assert_eq!(again.code().as_deref(), Some("account-two-step-code-wrong"));
    let code = user.authenticator.code_at(fake_now_ms(&served.clock))?;
    let first = step_two(&base, &challenge, &code, "replay-right").await?;
    assert_eq!(first.status, 200, "second try passes: {}", first.json);
    let replay = step_two(&base, &challenge, &code, "replay-right").await?;
    assert_eq!(replay.status, 200, "replayed success: {}", replay.json);
    assert_ne!(
        first.bearer()?,
        replay.bearer()?,
        "a replay opens a new session"
    );
    let tries: i32 = sqlx::query_scalar("SELECT tries FROM two_step_challenges")
        .fetch_one(&served.pool)
        .await
        .map_err(|e| e.to_string())?;
    assert_eq!(tries, 2, "replays do not use tries");
    served.stop().await
}

#[tokio::test]
async fn admin_password_reset_keeps_two_step_on() -> Result<(), String> {
    let (served, users) = served_with("two_step_admin_reset", &["alice", "bob"]).await?;
    let admin = users.first().ok_or("no admin")?;
    let base = served.base_url();
    let body = json!({"new": "bobnewpassword"});
    let path = "/v1/admin/accounts/bob/reset-password";
    let reset = signed_post(&base, &admin.bearer, path, &body, "admin-reset").await?;
    assert_eq!(reset.status, 200, "admin reset: {}", reset.json);
    let login = login_password(&base, "bob", "bobnewpassword", "bob-new").await?;
    assert_eq!(
        login.status, 202,
        "bob still needs the second step: {}",
        login.json
    );
    served.stop().await
}
