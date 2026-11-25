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

const N10_6_WAVE: usize = 30;
const N10_6_PAIR_LIMIT: usize = 8;
const N10_6_EXTRA: usize = 50;
const N10_6_DEADLINE: std::time::Duration = std::time::Duration::from_secs(10);
const N10_6_FAST_MS: u128 = 100;
const N10_6_ADDRESS: &str = "10.30.0.1";
const N10_6_WINDOW: std::time::Duration = std::time::Duration::from_secs(300);

fn login_body(account: &str, password: &str) -> String {
    json!({"account": account, "password": password, "label": "test-device"}).to_string()
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

async fn login_one(
    served: &anybot_app_test::serve::Served,
    account: &str,
    password: &str,
    key: &str,
) -> Result<ResponseSpec, String> {
    let base = served.base_url();
    let spec = login_spec(account, password, key).with_header(FORWARDED_HEADER, N10_6_ADDRESS);
    let clock = std::sync::Arc::clone(&served.clock);
    let mut handles = vec![tokio::spawn(Box::pin(side_post(base, spec)))];
    let answers = anybot_app_test::settle_handles(
        &clock,
        &mut handles,
        anybot_app_test::clockwalk::WAVE_FAKE_STEP,
        N10_6_DEADLINE,
    )
    .await?;
    answers
        .into_iter()
        .next()
        .ok_or("one handle must return one answer".to_owned())?
}

async fn side_post(base: String, spec: RequestSpec) -> Result<ResponseSpec, String> {
    post(&base, "/v1/account/login", &spec).await
}

async fn side_wave(base: String, index: usize) -> Result<ResponseSpec, String> {
    let spec = login_spec("dan", "wrongpassword", &format!("w{index}"))
        .with_header(FORWARDED_HEADER, N10_6_ADDRESS);
    post(&base, "/v1/account/login", &spec).await
}

#[tokio::test]
async fn t_safety_n10_6() -> Result<(), String> {
    let mut served = serve_start("t_safety_n10_6").await?;
    let invite = invite_of(&served.started)?;
    register(
        &served.base_url(),
        "alice",
        "alicepassword",
        &invite,
        "k-alice",
    )
    .await?;
    register(&served.base_url(), "dan", "danpassword", &invite, "k-dan").await?;
    served
        .set_config_and_restart(KEY_TRUST_PROXY, "127.0.0.1")
        .await?;
    let wave = run_wave(&served).await?;
    assert_verified(&served, wave).await?;
    let rejected = login_one(&served, "dan", "danpassword", "k-right-wave").await?;
    assert_eq!(
        rejected.status, 429,
        "right password body {}",
        rejected.json
    );
    let extras = run_extras(&served).await?;
    assert_fast_reject(&extras)?;
    let _advanced = served.clock.advance(N10_6_WINDOW);
    let after = login_one(&served, "dan", "danpassword", "k-after-window").await?;
    assert_eq!(after.status, 200, "after window body {}", after.json);
    served.stop().await?;
    Ok(())
}

async fn run_wave(served: &anybot_app_test::serve::Served) -> Result<usize, String> {
    let mut handles = Vec::new();
    for index in 0..N10_6_WAVE {
        let base = served.base_url();
        handles.push(tokio::spawn(Box::pin(side_wave(base, index))));
    }
    let clock = std::sync::Arc::clone(&served.clock);
    let answers = anybot_app_test::settle_handles(
        &clock,
        &mut handles,
        anybot_app_test::clockwalk::WAVE_FAKE_STEP,
        N10_6_DEADLINE,
    )
    .await?;
    tally(&answers)
}

fn tally(answers: &[Result<ResponseSpec, String>]) -> Result<usize, String> {
    let mut verified = 0usize;
    for answer in answers {
        let done = match answer.as_ref() {
            Ok(v) => v,
            Err(e) => return Err(e.clone()),
        };
        match done.status {
            401 => verified = verified.saturating_add(1),
            429 => {
                assert_eq!(
                    done.code().as_deref(),
                    Some("account-login-rate"),
                    "429 body {}",
                    done.json
                );
            }
            other => return Err(format!("wave status {other} body {}", done.json)),
        }
    }
    assert!(
        verified <= N10_6_PAIR_LIMIT,
        "{verified} passwords verified, above the pair limit"
    );
    Ok(verified)
}

async fn assert_verified(
    served: &anybot_app_test::serve::Served,
    verified: usize,
) -> Result<(), String> {
    let rows = anybot_store_server::auth_events::count_account(
        &served.pool,
        anybot_server::limits::KIND_LOGIN_FAIL,
        "dan",
        0,
    )
    .await
    .map_err(|e| e.to_string())?;
    assert_eq!(
        usize::try_from(rows).map_err(|e| e.to_string())?,
        verified,
        "recorded failures versus verified answers"
    );
    Ok(())
}

async fn run_extras(served: &anybot_app_test::serve::Served) -> Result<Vec<u128>, String> {
    let mut handles = Vec::new();
    for index in 0..N10_6_EXTRA {
        let base = served.base_url();
        handles.push(tokio::spawn(Box::pin(side_extra(base, index))));
    }
    let clock = std::sync::Arc::clone(&served.clock);
    let answers = anybot_app_test::settle_handles(
        &clock,
        &mut handles,
        anybot_app_test::clockwalk::WAVE_FAKE_STEP,
        N10_6_DEADLINE,
    )
    .await?;
    let mut times = Vec::new();
    for answer in answers {
        let done = match answer {
            Ok(v) => v,
            Err(e) => return Err(e),
        };
        assert_eq!(done.status, 429, "extra body {}", done.json);
        times.push(done.elapsed.as_millis());
    }
    Ok(times)
}

async fn side_extra(base: String, index: usize) -> Result<ResponseSpec, String> {
    let spec = login_spec("dan", "wrongpassword", &format!("x{index}"))
        .with_header(FORWARDED_HEADER, N10_6_ADDRESS);
    post(&base, "/v1/account/login", &spec).await
}

fn assert_fast_reject(times: &[u128]) -> Result<(), String> {
    let mid = median(times)?;
    assert!(
        mid < N10_6_FAST_MS,
        "median of {} extras is {mid} ms",
        times.len()
    );
    Ok(())
}

fn median(values: &[u128]) -> Result<u128, String> {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let mid = sorted.len() / 2;
    sorted
        .get(mid)
        .copied()
        .ok_or(format!("no median at {mid}"))
}
