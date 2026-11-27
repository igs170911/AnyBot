use std::sync::Arc;

use serde_json::{Value, json};

use test_harness::FakeClock;

use crate::ClockWalker;
use crate::fake_totp::FakeAuthenticator;
use crate::http::{RequestSpec, ResponseSpec, post};

pub const TWO_STEP_LOGIN_PATH: &str = "/v1/account/login/two-step";
pub const TWO_STEP_SETUP_PATH: &str = "/v1/account/two-step/setup";
pub const TWO_STEP_CONFIRM_PATH: &str = "/v1/account/two-step/confirm";
pub const TWO_STEP_DISABLE_PATH: &str = "/v1/account/two-step/disable";

pub struct TwoStepUser {
    pub account: String,
    pub password: String,
    pub bearer: String,
    pub authenticator: FakeAuthenticator,
    pub secret_base32: String,
    pub recovery_codes: Vec<String>,
}

#[must_use]
pub fn fake_now_ms(clock: &Arc<FakeClock>) -> u64 {
    clock.now_ms()
}

pub async fn login_password(
    base: &str,
    account: &str,
    password: &str,
    key: &str,
) -> Result<ResponseSpec, String> {
    let body = json!({"account": account, "password": password, "label": "test-device", "credential": "bearer"});
    let spec = RequestSpec::new(&body.to_string()).with_key(key);
    post(base, "/v1/account/login", &spec).await
}

pub async fn walked_post(
    base: &str,
    clock: &Arc<FakeClock>,
    path: &str,
    spec: &RequestSpec,
) -> Result<ResponseSpec, String> {
    let mut walker = ClockWalker::start(clock);
    let outcome = post(base, path, spec).await;
    walker.stop().await?;
    outcome
}

pub async fn bearer_of(
    base: &str,
    account: &str,
    password: &str,
    key: &str,
) -> Result<String, String> {
    let answer = login_password(base, account, password, key).await?;
    if answer.status != 200 {
        return Err(format!(
            "login {account}: {} {}",
            answer.status, answer.json
        ));
    }
    answer.bearer()
}

#[must_use]
pub fn step_two_spec(challenge: &str, code: &str, key: &str) -> RequestSpec {
    let body = json!({"challenge": challenge, "code": code});
    RequestSpec::new(&body.to_string()).with_key(key)
}

pub async fn step_two(
    base: &str,
    challenge: &str,
    code: &str,
    key: &str,
) -> Result<ResponseSpec, String> {
    post(
        base,
        TWO_STEP_LOGIN_PATH,
        &step_two_spec(challenge, code, key),
    )
    .await
}

pub async fn step_two_walked(
    base: &str,
    clock: &Arc<FakeClock>,
    challenge: &str,
    code: &str,
    key: &str,
) -> Result<ResponseSpec, String> {
    let spec = step_two_spec(challenge, code, key);
    walked_post(base, clock, TWO_STEP_LOGIN_PATH, &spec).await
}

const TIMED_TICK_MS: u64 = 50;
const TIMED_ROUNDS: usize = 500;

pub async fn timed_post(
    base: &str,
    clock: &Arc<FakeClock>,
    path: &str,
    spec: &RequestSpec,
) -> Result<(ResponseSpec, u64), String> {
    let (owned_base, owned_path, spec) = (base.to_owned(), path.to_owned(), spec.clone());
    let task = tokio::spawn(async move { post(&owned_base, &owned_path, &spec).await });
    let mut advanced: u64 = 0;
    for _round in 0..TIMED_ROUNDS {
        test_harness::sleep_real(crate::http::STEP_REAL).await;
        if task.is_finished() {
            break;
        }
        let _woken = clock.advance(std::time::Duration::from_millis(TIMED_TICK_MS));
        advanced = advanced.saturating_add(TIMED_TICK_MS);
    }
    if !task.is_finished() {
        task.abort();
        return Err(format!(
            "{path} did not answer after {advanced} ms of fake time"
        ));
    }
    let answer = task.await.map_err(|e| e.to_string())??;
    Ok((answer, advanced))
}

pub fn wrong_code_for(authenticator: &FakeAuthenticator, now_ms: u64) -> Result<String, String> {
    let taken = [
        authenticator.previous_code_at(now_ms)?,
        authenticator.code_at(now_ms)?,
        authenticator.next_code_at(now_ms)?,
    ];
    for digit in ['0', '1', '2', '3'] {
        let candidate: String = std::iter::repeat_n(digit, 6).collect();
        if !taken.contains(&candidate) {
            return Ok(candidate);
        }
    }
    Err("no wrong code found".to_owned())
}

pub async fn signed_post(
    base: &str,
    bearer: &str,
    path: &str,
    body: &Value,
    key: &str,
) -> Result<ResponseSpec, String> {
    let spec = RequestSpec::new(&body.to_string())
        .with_key(key)
        .with_bearer(bearer);
    post(base, path, &spec).await
}

pub async fn enable_two_step(
    base: &str,
    clock: &Arc<FakeClock>,
    account: &str,
    password: &str,
) -> Result<TwoStepUser, String> {
    enable_two_step_at(base, account, password, || fake_now_ms(clock)).await
}

pub async fn enable_two_step_at(
    base: &str,
    account: &str,
    password: &str,
    now_ms: impl Fn() -> u64,
) -> Result<TwoStepUser, String> {
    let bearer = bearer_of(base, account, password, &format!("enable-login-{account}")).await?;
    let setup = signed_post(
        base,
        &bearer,
        TWO_STEP_SETUP_PATH,
        &json!({}),
        &format!("setup-{account}"),
    )
    .await?;
    if setup.status != 200 {
        return Err(format!("setup {account}: {} {}", setup.status, setup.json));
    }
    let secret_base32 = setup.field_str("secret")?;
    let authenticator = FakeAuthenticator::from_base32(&secret_base32)?;
    let recovery_codes = string_list(&setup.json, "recovery_codes")?;
    let code = authenticator.code_at(now_ms())?;
    let confirm = signed_post(
        base,
        &bearer,
        TWO_STEP_CONFIRM_PATH,
        &json!({"code": code}),
        &format!("confirm-{account}"),
    )
    .await?;
    if confirm.status != 200 {
        return Err(format!(
            "confirm {account}: {} {}",
            confirm.status, confirm.json
        ));
    }
    Ok(TwoStepUser {
        account: account.to_owned(),
        password: password.to_owned(),
        bearer,
        authenticator,
        secret_base32,
        recovery_codes,
    })
}

pub fn string_list(json: &Value, field: &str) -> Result<Vec<String>, String> {
    let items = json
        .get(field)
        .and_then(Value::as_array)
        .ok_or(format!("no list {field} in {json}"))?;
    items
        .iter()
        .map(|item| {
            item.as_str()
                .map(String::from)
                .ok_or(format!("non-text item in {field}"))
        })
        .collect()
}

pub async fn challenge_of(base: &str, user: &TwoStepUser, key: &str) -> Result<String, String> {
    let answer = login_password(base, &user.account, &user.password, key).await?;
    if answer.status != 202 {
        return Err(format!(
            "expected the second step: {} {}",
            answer.status, answer.json
        ));
    }
    answer.field_str("challenge")
}
