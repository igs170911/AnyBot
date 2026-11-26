use serde_json::json;

use anybot_server::Started;

use crate::http::{FORWARDED_HEADER, RequestSpec, ResponseSpec, post};

#[must_use = "the value is consumed by the caller"]
pub fn login_body(account: &str, password: &str) -> String {
    json!({"account": account, "password": password, "label": "test-device"}).to_string()
}

#[must_use = "the value is consumed by the caller"]
pub fn register_body(account: &str, password: &str, invite: &str) -> String {
    json!({"account": account, "password": password, "invite": invite}).to_string()
}

#[must_use = "the spec drives one call"]
pub fn login_spec(account: &str, password: &str, key: &str, address: &str) -> RequestSpec {
    RequestSpec::new(&login_body(account, password))
        .with_key(key)
        .with_header(FORWARDED_HEADER, address)
}

#[must_use = "the spec drives one call"]
pub fn wave_spec(index: usize, address: &str) -> RequestSpec {
    login_spec("dan", "wrongpassword", &format!("w{index}"), address)
}

#[must_use = "the spec drives one call"]
pub fn extra_spec(index: usize, address: &str) -> RequestSpec {
    login_spec("dan", "wrongpassword", &format!("x{index}"), address)
}

#[must_use = "the value is consumed by the caller"]
pub fn invite_of(started: &Started) -> Result<String, String> {
    started
        .new_invite
        .clone()
        .ok_or("fresh boot must draw an invite".to_owned())
}

pub async fn register(
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

pub fn tally(
    answers: &[Result<ResponseSpec, String>],
    wave: usize,
    verify_max: usize,
) -> Result<usize, String> {
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
        verified <= verify_max,
        "{verified} passwords verified, above the pair limit",
    );
    let mut rejected = 0usize;
    for answer in answers {
        let done = match answer {
            Ok(v) => v,
            Err(e) => return Err(e.clone()),
        };
        if done.status == 429 {
            rejected = rejected.saturating_add(1);
        }
    }
    assert_eq!(
        rejected,
        wave.saturating_sub(verified),
        "fast rejects below the wave remainder",
    );
    Ok(verified)
}

pub fn assert_constant_wait(times: &[u128], fast_ms: u128) -> Result<(), String> {
    let mid = median(times)?;
    assert!(
        mid < fast_ms,
        "median of {} extras is {mid} ms",
        times.len()
    );
    Ok(())
}

#[must_use = "the value is consumed by the caller"]
pub fn median(values: &[u128]) -> Result<u128, String> {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let mid = sorted.len() / 2;
    sorted
        .get(mid)
        .copied()
        .ok_or(format!("no median at {mid}"))
}
