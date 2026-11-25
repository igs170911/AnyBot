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

use anybot_app_test::http::{RequestSpec, ResponseSpec, post};
use anybot_app_test::n106::{extra_spec, invite_of, login_spec, register, tally, wave_spec};
use anybot_app_test::serve::serve_start;

use anybot_server::KEY_TRUST_PROXY;
use anybot_server::limits::KIND_LOGIN_FAIL;

const WAVE: usize = 30;
const VERIFY_MAX: usize = 10;
const EXTRA: usize = 50;
const DEADLINE: std::time::Duration = std::time::Duration::from_secs(15);
const FAST_MS: u128 = 1000;
const ADDRESS: &str = "10.30.0.1";
const WINDOW: std::time::Duration = std::time::Duration::from_secs(300);

async fn side_post(base: String, spec: RequestSpec) -> Result<ResponseSpec, String> {
    post(&base, "/v1/account/login", &spec).await
}

async fn login_one(
    served: &anybot_app_test::serve::Served,
    account: &str,
    password: &str,
    key: &str,
) -> Result<ResponseSpec, String> {
    let base = served.base_url();
    let spec = login_spec(account, password, key, ADDRESS);
    let clock = std::sync::Arc::clone(&served.clock);
    let mut handles = vec![tokio::spawn(Box::pin(side_post(base, spec)))];
    let answers = anybot_app_test::settle_handles(
        &clock,
        &mut handles,
        anybot_app_test::WAVE_FAKE_STEP,
        DEADLINE,
    )
    .await?;
    answers
        .into_iter()
        .next()
        .ok_or("one handle must return one answer".to_owned())?
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
    let pool = std::sync::Arc::new(anybot_app_test::http::ConnectionPool::new(WAVE)?);
    warm_up(&served, &pool).await?;
    let wave = run_wave(&served, &pool).await?;
    assert_verified(&served, wave).await?;
    let rejected = login_one(&served, "dan", "danpassword", "k-right-wave").await?;
    assert_eq!(
        rejected.status, 429,
        "right password body {}",
        rejected.json
    );
    let extras = run_extras(&served, &pool).await?;
    anybot_app_test::n106::assert_constant_wait(&extras, FAST_MS)?;
    let _advanced = served.clock.advance(WINDOW);
    let after = login_one(&served, "dan", "danpassword", "k-after-window").await?;
    assert_eq!(after.status, 200, "after window body {}", after.json);
    served.stop().await?;
    Ok(())
}

async fn assert_verified(
    served: &anybot_app_test::serve::Served,
    verified: usize,
) -> Result<(), String> {
    let rows =
        anybot_store_server::auth_events::count_account(&served.pool, KIND_LOGIN_FAIL, "dan", 0)
            .await
            .map_err(|e| e.to_string())?;
    assert_eq!(
        usize::try_from(rows).map_err(|e| e.to_string())?,
        verified,
        "recorded failures versus verified answers"
    );
    Ok(())
}

async fn warm_up(
    served: &anybot_app_test::serve::Served,
    pool: &std::sync::Arc<anybot_app_test::http::ConnectionPool>,
) -> Result<(), String> {
    let base = served.base_url();
    let mut handles = Vec::new();
    for _index in 0..WAVE {
        let owned = std::sync::Arc::clone(pool);
        let site = base.clone();
        handles.push(tokio::spawn(async move {
            owned.get(&site, "/v1/health", &RequestSpec::new("")).await
        }));
    }
    let clock = std::sync::Arc::clone(&served.clock);
    let answers = anybot_app_test::settle_handles(
        &clock,
        &mut handles,
        anybot_app_test::WAVE_INSERT_STEP,
        DEADLINE,
    )
    .await?;
    for answer in answers {
        match answer {
            Ok(v) if v.status == 200 => {}
            Ok(v) => return Err(format!("warm status {}", v.status)),
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

async fn run_wave(
    served: &anybot_app_test::serve::Served,
    pool: &std::sync::Arc<anybot_app_test::http::ConnectionPool>,
) -> Result<usize, String> {
    let mut handles = Vec::new();
    for index in 0..WAVE {
        let base = served.base_url();
        let owned = std::sync::Arc::clone(pool);
        let spec = wave_spec(index, ADDRESS);
        handles.push(tokio::spawn(side_send(base, spec, owned)));
    }
    let clock = std::sync::Arc::clone(&served.clock);
    let settled = anybot_app_test::settle_handles(
        &clock,
        &mut handles,
        anybot_app_test::WAVE_INSERT_STEP,
        DEADLINE,
    )
    .await?;
    tally(&settled, WAVE, VERIFY_MAX)
}

async fn run_extras(
    served: &anybot_app_test::serve::Served,
    pool: &std::sync::Arc<anybot_app_test::http::ConnectionPool>,
) -> Result<Vec<u128>, String> {
    let mut handles = Vec::new();
    for index in 0..EXTRA {
        let base = served.base_url();
        let owned = std::sync::Arc::clone(pool);
        let spec = extra_spec(index, ADDRESS);
        handles.push(tokio::spawn(side_send(base, spec, owned)));
    }
    let clock = std::sync::Arc::clone(&served.clock);
    let settled = anybot_app_test::settle_handles(
        &clock,
        &mut handles,
        anybot_app_test::WAVE_INSERT_STEP,
        DEADLINE,
    )
    .await?;
    let mut times = Vec::new();
    for answer in settled {
        let done = match answer {
            Ok(v) => v,
            Err(e) => return Err(e),
        };
        assert_eq!(done.status, 429, "extra body {}", done.json);
        times.push(done.elapsed.as_millis());
    }
    Ok(times)
}

async fn side_send(
    base: String,
    spec: RequestSpec,
    pool: std::sync::Arc<anybot_app_test::http::ConnectionPool>,
) -> Result<ResponseSpec, String> {
    pool.post(&base, "/v1/account/login", &spec).await
}
