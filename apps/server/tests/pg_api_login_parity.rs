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
use anybot_app_test::serve::serve_start_real;

use anybot_server::KEY_TRUST_PROXY;

const N10_2_ADDRESSES: usize = 6;
const N10_2_PER_ADDRESS: usize = 7;
const N10_2_EXPECTED_401: usize = 40;
const N10_2_EXPECTED_429: usize = 2;
const N10_2_MIN_MS: u128 = 500;
const N10_2_PARITY_ROUNDS: usize = 3;
const N10_2_PARITY_GAP_MS: u128 = 50;

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

#[tokio::test]
async fn t_safety_n10_2() -> Result<(), String> {
    let mut served = serve_start_real("t_safety_n10_2").await?;
    let invite = invite_of(&served.started)?;
    register(&served.base_url(), "dan", "danpassword", &invite, "k-dan").await?;
    register(
        &served.base_url(),
        "erin",
        "erinpassword",
        &invite,
        "k-erin",
    )
    .await?;
    served
        .set_config_and_restart(KEY_TRUST_PROXY, "127.0.0.1")
        .await?;
    let wave = run_wave(&served).await?;
    assert_eq!(wave.0, N10_2_EXPECTED_401, "verified failures");
    assert_eq!(wave.1, N10_2_EXPECTED_429, "blocked attempts");
    parity_within_fifty_ms(&served).await?;
    served.stop().await?;
    Ok(())
}

async fn run_wave(served: &anybot_app_test::serve::ServedReal) -> Result<(usize, usize), String> {
    let mut tasks: Vec<tokio::task::JoinHandle<Result<Vec<ResponseSpec>, String>>> = Vec::new();
    for index in 0..N10_2_ADDRESSES {
        let base = served.base_url();
        let address = format!("10.20.0.{index}");
        let side = wave_side(base, address, index);
        tasks.push(tokio::spawn(Box::pin(side)));
    }
    let mut verified = 0usize;
    let mut blocked = 0usize;
    for task in tasks {
        for answer in task.await.map_err(|e| e.to_string())?? {
            assert!(
                answer.elapsed.as_millis() >= N10_2_MIN_MS,
                "dan attempt took {} ms",
                answer.elapsed.as_millis()
            );
            match answer.status {
                401 => {
                    assert_eq!(
                        answer.code().as_deref(),
                        Some("account-login-credentials"),
                        "401 body {}",
                        answer.json
                    );
                    verified = verified.saturating_add(1);
                }
                429 => {
                    assert_eq!(
                        answer.code().as_deref(),
                        Some("account-login-rate"),
                        "429 body {}",
                        answer.json
                    );
                    blocked = blocked.saturating_add(1);
                }
                other => {
                    return Err(format!("dan attempt status {other} body {}", answer.json));
                }
            }
        }
    }
    Ok((verified, blocked))
}

async fn wave_side(
    base: String,
    address: String,
    index: usize,
) -> Result<Vec<ResponseSpec>, String> {
    let mut answers = Vec::new();
    for round in 0..N10_2_PER_ADDRESS {
        let spec = login_spec("dan", "wrongpassword", &format!("n{index}r{round}"))
            .with_header(FORWARDED_HEADER, &address);
        answers.push(post(&base, "/v1/account/login", &spec).await?);
    }
    Ok(answers)
}

async fn parity_within_fifty_ms(served: &anybot_app_test::serve::ServedReal) -> Result<(), String> {
    let mut existing = Vec::new();
    let mut ghost = Vec::new();
    for round in 1..=N10_2_PARITY_ROUNDS {
        let answer = post(
            &served.base_url(),
            "/v1/account/login",
            &login_spec("erin", "wrongpassword", &format!("p{round}")),
        )
        .await?;
        assert_eq!(answer.status, 401, "existing answer body {}", answer.json);
        existing.push(answer.elapsed.as_millis());
        let ghost_answer = post(
            &served.base_url(),
            "/v1/account/login",
            &login_spec("ghostname", "anypassword", &format!("q{round}")),
        )
        .await?;
        assert_eq!(
            ghost_answer.status, 401,
            "ghost answer body {}",
            ghost_answer.json
        );
        assert_eq!(
            ghost_answer.code().as_deref(),
            Some("account-login-credentials"),
        );
        ghost.push(ghost_answer.elapsed.as_millis());
    }
    let gap = median(&existing)?.abs_diff(median(&ghost)?);
    assert!(gap < N10_2_PARITY_GAP_MS, "ghost parity gap {gap} ms");
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
