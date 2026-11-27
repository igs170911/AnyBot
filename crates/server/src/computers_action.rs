use axum::extract::Request;
use axum::http::StatusCode;
use axum::response::Response;

use anybot_domain::computer_actions::{AccountAction, paired_may};
use anybot_messages::MessageCode;

use crate::api_ops::{guard_replay, record};
use crate::app::AppState;
use crate::caller::Caller;
use crate::req_write::{CallerDenied, caller_scope, denied, idempotency_key, pool_of, read_json};

pub(crate) struct Prepared<T> {
    pub key: String,
    pub scope: String,
    pub raw: Vec<u8>,
    pub body: T,
}

pub(crate) enum Step<T> {
    Replay(Response),
    Run(Prepared<T>),
}

pub(crate) fn require_allowed(caller: &Caller, action: AccountAction) -> Result<(), CallerDenied> {
    if caller.paired && !paired_may(action) {
        return Err(denied(
            StatusCode::FORBIDDEN,
            MessageCode::ComputerPairedForbidden,
        ));
    }
    Ok(())
}

pub(crate) async fn prepare<T: serde::de::DeserializeOwned>(
    state: &AppState,
    caller: &Caller,
    request: Request,
) -> Result<Step<T>, CallerDenied> {
    let key = idempotency_key(&request)?;
    let scope = caller_scope(&request, &caller.account);
    let (body, raw) = read_json::<T>(request).await?;
    let pool = pool_of(state)?;
    if let Some(replay) = guard_replay(pool, &key, &raw, &scope).await? {
        return Ok(Step::Replay(replay));
    }
    Ok(Step::Run(Prepared {
        key,
        scope,
        raw,
        body,
    }))
}

pub(crate) async fn finish<T>(
    state: &AppState,
    prepared: &Prepared<T>,
    status: StatusCode,
    value: &serde_json::Value,
) -> Result<Response, CallerDenied> {
    let pool = pool_of(state)?;
    record(
        pool,
        state,
        &prepared.key,
        &prepared.raw,
        &prepared.scope,
        status,
        value,
    )
    .await
}
