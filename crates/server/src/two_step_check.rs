use std::sync::Arc;

use axum::http::StatusCode;

use anybot_domain::two_step::{CodeInput, classify_code, matching_step, reuse_floor};
use anybot_foundation::{Secret, SecretBytes};
use anybot_messages::MessageCode;
use anybot_store_server::two_step::TwoStepStoreError;
use anybot_store_server::two_step_codes::{claim_step, use_recovery_code};
use anybot_store_server::{Pool, SecretKeys};

use crate::app::AppState;
use crate::caller::CallerDenied;
use crate::req_write::{denied, internal_detail, internal_sqlx, internal_text};

const WRONG_CODE_DELAY_MS: u64 = 300;

pub(crate) fn secrets_of(state: &AppState) -> Result<Arc<SecretKeys>, CallerDenied> {
    state
        .secrets
        .clone()
        .ok_or_else(|| internal_detail("secret keys are not configured"))
}

pub(crate) fn now_ms(state: &AppState) -> Result<i64, CallerDenied> {
    i64::try_from(state.clock.now_ms().millis()).map_err(internal_text)
}

pub(crate) fn opened(
    account: &str,
    result: Result<Option<SecretBytes>, TwoStepStoreError>,
) -> Result<Option<SecretBytes>, CallerDenied> {
    result.map_err(|err| unreadable(account, &err))
}

pub(crate) fn unreadable(account: &str, err: &TwoStepStoreError) -> CallerDenied {
    tracing::error!(account, reason = %err, "two-step secret could not be opened");
    denied(
        StatusCode::INTERNAL_SERVER_ERROR,
        MessageCode::AccountTwoStepUnreadable,
    )
}

pub(crate) fn code_in_window(
    state: &AppState,
    secret: &SecretBytes,
    code: &str,
) -> Result<Option<i64>, CallerDenied> {
    let CodeInput::Totp(digits) = classify_code(code) else {
        return Ok(None);
    };
    matching_step(secret.expose(), now_ms(state)?, &digits)
        .map_err(|_| internal_detail("time step out of range"))
}

pub(crate) struct LoginCode<'a> {
    pub state: &'a AppState,
    pub pool: &'a Pool,
    pub keys: &'a SecretKeys,
    pub account: &'a str,
    pub secret: &'a SecretBytes,
    pub code: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AcceptedCode {
    Totp,
    Recovery,
}

pub(crate) async fn login_code_passes(
    check: &LoginCode<'_>,
) -> Result<Option<AcceptedCode>, CallerDenied> {
    let now = now_ms(check.state)?;
    match classify_code(check.code) {
        CodeInput::Totp(_) => {
            let Some(step) = code_in_window(check.state, check.secret, check.code)? else {
                return Ok(None);
            };
            let claimed = claim_step(check.pool, check.account, step, now, reuse_floor(now))
                .await
                .map_err(internal_sqlx)?;
            Ok(claimed.then_some(AcceptedCode::Totp))
        }
        CodeInput::Recovery(text) => {
            let code = Secret::from_untrusted(text);
            let used = use_recovery_code(check.pool, check.keys, check.account, &code, now)
                .await
                .map_err(|err| unreadable(check.account, &err))?;
            Ok(used.then_some(AcceptedCode::Recovery))
        }
        CodeInput::Malformed => Ok(None),
    }
}

pub(crate) async fn wrong_code_pause(state: &AppState) {
    state
        .clock
        .sleep(std::time::Duration::from_millis(WRONG_CODE_DELAY_MS))
        .await;
}
