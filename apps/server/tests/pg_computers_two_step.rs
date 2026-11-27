use anybot_app_server as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_messages as _;
use anybot_server as _;
use anybot_store_server as _;
use axum as _;
use clap as _;
use rpassword as _;
use sqlx as _;
use test_harness as _;
use tokio as _;
use tracing as _;

use serde_json::json;

use anybot_app_test::http::{RequestSpec, post};
use anybot_app_test::n106::{invite_of, register};
use anybot_app_test::serve::serve_start;
use anybot_app_test::two_step_client::{
    TWO_STEP_LOGIN_PATH, challenge_of, enable_two_step, fake_now_ms, step_two_spec,
};
use anybot_server_test::computers_support::{api_get, api_post};

#[tokio::test]
async fn second_step_issues_a_computer_credential_when_asked() -> Result<(), String> {
    let served = serve_start("two_step_computer").await?;
    let base = served.base_url();
    let invite = invite_of(&served.started)?;
    register(&base, "alice", "alicepassword", &invite, "k-alice").await?;
    let user = enable_two_step(&base, &served.clock, "alice", "alicepassword").await?;
    let body = json!({"account": "alice", "password": "alicepassword", "label": "study", "credential": "computer"});
    let first = post(
        &base,
        "/v1/account/login",
        &RequestSpec::new(&body.to_string()).with_key("computer-first"),
    )
    .await?;
    assert_eq!(first.status, 202, "first step: {}", first.json);
    let challenge = first.field_str("challenge")?;
    let code = user
        .authenticator
        .code_at(fake_now_ms(&served.clock) + 30_000)?;
    let second = post(
        &base,
        TWO_STEP_LOGIN_PATH,
        &step_two_spec(&challenge, &code, "computer-second"),
    )
    .await?;
    assert_eq!(second.status, 200, "second step: {}", second.json);
    let bearer = second.bearer()?;
    let listed = api_get(&served, &bearer, "/v1/computers").await?;
    assert_eq!(listed.status, 200, "{}", listed.json);
    let rows = listed
        .json
        .get("computers")
        .and_then(|rows| rows.as_array());
    assert_eq!(rows.map(Vec::len), Some(1), "{}", listed.json);
    let again = challenge_of(&base, &user, "computer-unused").await;
    assert!(again.is_ok(), "{again:?}");
    let leave = api_post(
        &served,
        &bearer,
        "/v1/account/logout",
        &json!({"ok": true}),
        "k-leave",
    )
    .await?;
    assert_eq!(leave.status, 200, "{}", leave.json);
    let after = api_get(&served, &bearer, "/v1/computers").await?;
    assert_eq!(after.status, 401, "{}", after.json);
    served.stop().await?;
    Ok(())
}
