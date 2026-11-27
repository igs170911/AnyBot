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
    clock: &Arc<FakeClock>,
    account: &str,
    password: &str,
    key: &str,
) -> Result<ResponseSpec, String> {
    let body = json!({"account": account, "password": password, "label": "test-device", "credential": "bearer"});
    let spec = RequestSpec::new(&body.to_string()).with_key(key);
    walked_post(base, clock, "/v1/account/login", &spec).await
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
    clock: &Arc<FakeClock>,
    account: &str,
    password: &str,
    key: &str,
) -> Result<String, String> {
    let answer = login_password(base, clock, account, password, key).await?;
    if answer.status != 200 {
        return Err(format!(
            "login {account}: {} {}",
            answer.status, answer.json
        ));
    }
    answer.bearer()
}

pub async fn step_two(
    base: &str,
    clock: &Arc<FakeClock>,
    challenge: &str,
    code: &str,
    key: &str,
) -> Result<ResponseSpec, String> {
    let body = json!({"challenge": challenge, "code": code});
    let spec = RequestSpec::new(&body.to_string()).with_key(key);
    walked_post(base, clock, TWO_STEP_LOGIN_PATH, &spec).await
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
    let bearer = bearer_of(
        base,
        clock,
        account,
        password,
        &format!("enable-login-{account}"),
    )
    .await?;
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
    let code = authenticator.code_at(fake_now_ms(clock))?;
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

pub async fn challenge_of(
    base: &str,
    clock: &Arc<FakeClock>,
    user: &TwoStepUser,
    key: &str,
) -> Result<String, String> {
    let answer = login_password(base, clock, &user.account, &user.password, key).await?;
    if answer.status != 202 {
        return Err(format!(
            "expected the second step: {} {}",
            answer.status, answer.json
        ));
    }
    answer.field_str("challenge")
}
