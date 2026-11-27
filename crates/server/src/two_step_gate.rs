use axum::http::StatusCode;

use anybot_domain::two_step::{
    RECOVERY_CODE_BYTES, RECOVERY_CODE_COUNT, SECRET_BYTES, otpauth_uri, recovery_code,
    secret_base32,
};
use anybot_foundation::{Secret, SecretBytes};
use anybot_messages::MessageCode;
use anybot_protocol::TwoStepSetup;
use anybot_store_server::two_step::{open_active_secret, open_pending_secret};
use anybot_store_server::{Pool, SecretKeys};

use crate::app::AppState;
use crate::caller::{Caller, CallerDenied};
use crate::limits::Limiter;
use crate::req_write::{denied, internal_detail, internal_sqlx};
use crate::two_step_check::{code_in_window, opened};

pub(crate) const EMPTY_FINGERPRINT: &[u8] = b"{}";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SecretSlot {
    Active,
    Pending,
}

pub(crate) struct CodeGate<'a> {
    pub state: &'a AppState,
    pub pool: &'a Pool,
    pub keys: &'a SecretKeys,
    pub caller: &'a Caller,
    pub code: Option<&'a str>,
    pub slot: SecretSlot,
}

pub(crate) async fn require_code(gate: &CodeGate<'_>) -> Result<SecretBytes, CallerDenied> {
    let limiter = Limiter::new(gate.pool, std::sync::Arc::clone(&gate.state.clock));
    let admitted = limiter
        .begin_two_step(&gate.caller.address, &gate.caller.account)
        .await
        .map_err(internal_sqlx)?;
    let Some(attempt) = admitted else {
        return Err(denied(
            StatusCode::TOO_MANY_REQUESTS,
            MessageCode::AccountTwoStepRate,
        ));
    };
    let checked = checked_secret(gate).await;
    if !matches!(checked, Ok(Checked::Wrong)) {
        limiter.cancel_login(attempt).await.map_err(internal_sqlx)?;
    }
    match checked? {
        Checked::Right(secret) => Ok(secret),
        Checked::Missing => Err(denied(
            StatusCode::BAD_REQUEST,
            MessageCode::AccountTwoStepNoPending,
        )),
        Checked::Wrong => Err(denied(
            StatusCode::UNAUTHORIZED,
            MessageCode::AccountTwoStepSetupCodeWrong,
        )),
    }
}

enum Checked {
    Right(SecretBytes),
    Wrong,
    Missing,
}

async fn checked_secret(gate: &CodeGate<'_>) -> Result<Checked, CallerDenied> {
    let account = gate.caller.account.as_str();
    let result = match gate.slot {
        SecretSlot::Active => open_active_secret(gate.pool, gate.keys, account).await,
        SecretSlot::Pending => open_pending_secret(gate.pool, gate.keys, account).await,
    };
    let Some(secret) = opened(account, result)? else {
        return Ok(Checked::Missing);
    };
    let Some(code) = gate.code else {
        return Ok(Checked::Wrong);
    };
    if code_in_window(gate.state, &secret, code)?.is_some() {
        return Ok(Checked::Right(secret));
    }
    Ok(Checked::Wrong)
}

pub(crate) struct FreshSetup {
    pub secret: SecretBytes,
    pub recovery_codes: Vec<Secret>,
    pub shown: TwoStepSetup,
}

pub(crate) fn fresh_setup(account: &str) -> Result<FreshSetup, CallerDenied> {
    let mut secret = vec![0u8; SECRET_BYTES];
    getrandom::fill(&mut secret).map_err(|e| internal_detail(&e.to_string()))?;
    let mut shown_codes = Vec::new();
    for _ in 0..RECOVERY_CODE_COUNT {
        let mut bytes = [0u8; RECOVERY_CODE_BYTES];
        getrandom::fill(&mut bytes).map_err(|e| internal_detail(&e.to_string()))?;
        shown_codes.push(recovery_code(&bytes));
    }
    let shown = TwoStepSetup {
        otpauth_uri: otpauth_uri(account, &secret),
        secret: secret_base32(&secret),
        recovery_codes: shown_codes.clone(),
    };
    Ok(FreshSetup {
        secret: SecretBytes::from_untrusted(secret),
        recovery_codes: shown_codes
            .into_iter()
            .map(Secret::from_untrusted)
            .collect(),
        shown,
    })
}
