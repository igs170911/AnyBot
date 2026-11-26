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
use sqlx::Row as _;
use test_harness as _;
use tokio as _;
use tracing as _;

use anybot_app_test::http::{RequestSpec, ResponseSpec, post};
use anybot_app_test::serve::serve_start;

fn login_body(account: &str, password: &str) -> String {
    json!({"account": account, "password": password, "label": "test-device"}).to_string()
}

fn register_body(account: &str, password: &str, invite: &str) -> String {
    json!({"account": account, "password": password, "invite": invite}).to_string()
}

async fn post_login(
    served: &anybot_app_test::serve::Served,
    account: &str,
    password: &str,
    key: &str,
) -> Result<ResponseSpec, String> {
    let mut walker = anybot_app_test::ClockWalker::start(&served.clock);
    let outcome = post(
        &served.base_url(),
        "/v1/account/login",
        &RequestSpec::new(&login_body(account, password)).with_key(key),
    )
    .await?;
    walker.stop().await?;
    Ok(outcome)
}

#[tokio::test]
async fn login_case_variants_share_one_counter() -> Result<(), String> {
    let served = serve_start("login_case_variants").await?;
    let invite = served
        .started
        .new_invite
        .clone()
        .ok_or("fresh boot must draw an invite")?;
    let created = post(
        &served.base_url(),
        "/v1/account/register",
        &RequestSpec::new(&register_body("bob", "bobpassword", &invite)).with_key("k-bob"),
    )
    .await?;
    assert_eq!(created.status, 201, "{}", created.json);
    let spellings = ["bob", "Bob", "bOb", "BOb", "boB", "BoB", "bOB", "BOB"];
    for (round, spelling) in spellings.iter().enumerate() {
        let fail = post_login(&served, spelling, "wrongpassword", &format!("case-{round}")).await?;
        assert_eq!(fail.status, 401, "spelling {spelling} round {round}");
    }
    let ninth = post_login(&served, "bob", "bobpassword", "case-ninth").await?;
    assert_eq!(ninth.status, 429, "mixed case must share the 8-try cap");
    assert_eq!(ninth.code().as_deref(), Some("account-login-rate"));
    let rows = sqlx::query("SELECT DISTINCT account FROM auth_events WHERE kind = 'login-fail'")
        .fetch_all(&served.pool)
        .await
        .map_err(|e| e.to_string())?;
    let names: Vec<String> = rows
        .iter()
        .map(|row| row.try_get("account").map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(names, vec!["bob".to_owned()]);
    served.stop().await?;
    Ok(())
}
