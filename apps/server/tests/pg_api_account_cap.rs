use std::sync::Arc;

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

use anybot_app_test::clockwalk::{WAVE_FAKE_STEP, settle_handles};
use anybot_app_test::http::{FORWARDED_HEADER, RequestSpec, ResponseSpec, post};
use anybot_app_test::serve::serve_start;
use anybot_server::KEY_TRUST_PROXY;

const ACCOUNT_CAP: usize = 40;
const BURST: usize = 48;
const GATE: &str = "login-fail|account|dan";

fn login_body(account: &str, password: &str) -> String {
    json!({"account": account, "password": password, "label": "test-device"}).to_string()
}

fn register_body(account: &str, password: &str, invite: &str) -> String {
    json!({"account": account, "password": password, "invite": invite}).to_string()
}

async fn one_login(
    base: String,
    address: String,
    index: usize,
    start: Arc<tokio::sync::Barrier>,
) -> Result<ResponseSpec, String> {
    start.wait().await;
    post(
        &base,
        "/v1/account/login",
        &RequestSpec::new(&login_body("dan", "wrongpassword"))
            .with_key(&format!("cap-{index}"))
            .with_header(FORWARDED_HEADER, &address),
    )
    .await
}

#[tokio::test]
async fn concurrent_logins_from_distinct_addresses_stop_at_the_account_cap() -> Result<(), String> {
    let mut served = serve_start("account_cap_burst").await?;
    let invite = served
        .started
        .new_invite
        .clone()
        .ok_or("fresh boot must draw an invite")?;
    let created = post(
        &served.base_url(),
        "/v1/account/register",
        &RequestSpec::new(&register_body("dan", "danpassword", &invite)).with_key("k-dan"),
    )
    .await?;
    assert_eq!(created.status, 201, "{}", created.json);
    served
        .set_config_and_restart(KEY_TRUST_PROXY, "127.0.0.1")
        .await?;
    let mut gate = served.pool.acquire().await.map_err(|e| e.to_string())?;
    sqlx::query("SELECT pg_advisory_lock(hashtext($1))")
        .bind(GATE)
        .execute(&mut *gate)
        .await
        .map_err(|e| e.to_string())?;
    let base = served.base_url();
    let start = Arc::new(tokio::sync::Barrier::new(BURST));
    let mut tasks = Vec::new();
    for index in 0..BURST {
        let address = format!("10.40.0.{index}");
        tasks.push(tokio::spawn(one_login(
            base.clone(),
            address,
            index,
            Arc::clone(&start),
        )));
    }
    test_harness::sleep_real(std::time::Duration::from_millis(800)).await;
    sqlx::query("SELECT pg_advisory_unlock(hashtext($1))")
        .bind(GATE)
        .execute(&mut *gate)
        .await
        .map_err(|e| e.to_string())?;
    drop(gate);
    let answers = settle_handles(
        &served.clock,
        &mut tasks,
        WAVE_FAKE_STEP,
        std::time::Duration::from_secs(30),
    )
    .await?;
    count_cap(&served, answers).await?;
    served.stop().await?;
    Ok(())
}

async fn count_cap(
    served: &anybot_app_test::serve::Served,
    answers: Vec<Result<ResponseSpec, String>>,
) -> Result<(), String> {
    let mut accepted = 0usize;
    let mut blocked = 0usize;
    for answer in answers {
        let answer = answer?;
        match answer.status {
            401 => {
                assert_eq!(answer.code().as_deref(), Some("account-login-credentials"));
                accepted = accepted.saturating_add(1);
            }
            429 => {
                assert_eq!(answer.code().as_deref(), Some("account-login-rate"));
                blocked = blocked.saturating_add(1);
            }
            other => return Err(format!("status {other} body {}", answer.json)),
        }
    }
    assert_eq!(accepted, ACCOUNT_CAP, "accepted must equal the account cap");
    assert_eq!(blocked, BURST.saturating_sub(ACCOUNT_CAP));
    let row = sqlx::query(
        "SELECT count(*) AS total FROM auth_events WHERE kind = 'login-fail' AND account = 'dan'",
    )
    .fetch_one(&served.pool)
    .await
    .map_err(|e| e.to_string())?;
    let total: i64 = row.try_get("total").map_err(|e| e.to_string())?;
    assert_eq!(
        total,
        i64::try_from(ACCOUNT_CAP).map_err(|e| e.to_string())?
    );
    Ok(())
}
