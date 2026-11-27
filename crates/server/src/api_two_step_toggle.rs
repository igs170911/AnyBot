use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;

use anybot_protocol::{ActionAck, ProblemJson, TwoStepCodeRequest};
use anybot_store_server::two_step::{clear_two_step, confirm_pending, two_step_enabled};

use crate::api_ops::{guard_replay, record};
use crate::app::AppState;
use crate::caller::{Caller, CallerDenied};
use crate::req_write::{
    caller_scope, idempotency_key, internal_sqlx, pool_of, read_json, value_of,
};
use crate::two_step_check::{now_ms, secrets_of, unreadable};
use crate::two_step_gate::{CodeGate, EMPTY_FINGERPRINT, SecretSlot, require_code};

#[utoipa::path(
    post,
    path = "/v1/account/two-step/confirm",
    operation_id = "post_account_two_step_confirm",
    summary = "Confirm the two-step setup",
    description = "Turns two-step verification on with the pending secret once the current code from it is given; the recovery codes of that setup replace the old ones. Requires the Idempotency-Key header.",
    tag = "account",
    security(("bearer" = [])),
    request_body(content = TwoStepCodeRequest, example = json!({"code": "123456"})),
    responses(
        (status = 200, description = "Two-step verification is on",
            content_type = "application/json",
            body = ActionAck,
            example = json!({"ok": true})),
        (status = 400, description = "No setup is waiting",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"code": "account-two-step-no-pending", "params": {}, "status": 400, "title": "There is no setup waiting for confirmation.", "type": "https://anybot.test/problems/account_two_step_no_pending"})),
        (status = 401, description = "Code missing or wrong",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"code": "account-two-step-setup-code-wrong", "params": {}, "status": 401, "title": "The verification code is wrong.", "type": "https://anybot.test/problems/account_two_step_setup_code_wrong"})),
        (status = 429, description = "Too many wrong codes",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"code": "account-two-step-rate", "params": {}, "status": 429, "title": "Too many wrong verification codes; try again in 5 minutes.", "type": "https://anybot.test/problems/account_two_step_rate"})),
        (status = 500, description = "Internal error",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"code": "problem-internal", "params": {}, "status": 500, "title": "The server hit an internal error.", "type": "https://anybot.test/problems/problem_internal"})),
    ),
    params(
        ("Idempotency-Key" = String, Header, description = "client-generated idempotency key")
    )
)]
pub async fn confirm_two_step(
    State(state): State<AppState>,
    caller: Caller,
    request: axum::extract::Request,
) -> Result<Response, CallerDenied> {
    let key = idempotency_key(&request)?;
    let scope = caller_scope(&request, &caller.account);
    let (body, _raw) = read_json::<TwoStepCodeRequest>(request).await?;
    let pool = pool_of(&state)?;
    if let Some(replay) = guard_replay(pool, &key, EMPTY_FINGERPRINT, &scope).await? {
        return Ok(replay);
    }
    let keys = secrets_of(&state)?;
    let gate = CodeGate {
        state: &state,
        pool,
        keys: &keys,
        caller: &caller,
        code: body.code.as_deref(),
        slot: SecretSlot::Pending,
    };
    let secret = require_code(&gate).await?;
    confirm_pending(pool, &keys, &caller.account, &secret, now_ms(&state)?)
        .await
        .map_err(|err| unreadable(&caller.account, &err))?;
    let ok = value_of(&ActionAck { ok: true })?;
    record(
        pool,
        &state,
        &key,
        EMPTY_FINGERPRINT,
        &scope,
        StatusCode::OK,
        &ok,
    )
    .await
}

#[utoipa::path(
    post,
    path = "/v1/account/two-step/disable",
    operation_id = "post_account_two_step_disable",
    summary = "Turn two-step verification off",
    description = "Requires the current code; the secret and the recovery codes are removed. Answers ok when two-step was already off. Requires the Idempotency-Key header.",
    tag = "account",
    security(("bearer" = [])),
    request_body(content = TwoStepCodeRequest, example = json!({"code": "123456"})),
    responses(
        (status = 200, description = "Two-step verification is off",
            content_type = "application/json",
            body = ActionAck,
            example = json!({"ok": true})),
        (status = 401, description = "Code missing or wrong",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"code": "account-two-step-setup-code-wrong", "params": {}, "status": 401, "title": "The verification code is wrong.", "type": "https://anybot.test/problems/account_two_step_setup_code_wrong"})),
        (status = 429, description = "Too many wrong codes",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"code": "account-two-step-rate", "params": {}, "status": 429, "title": "Too many wrong verification codes; try again in 5 minutes.", "type": "https://anybot.test/problems/account_two_step_rate"})),
        (status = 500, description = "Internal error",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"code": "problem-internal", "params": {}, "status": 500, "title": "The server hit an internal error.", "type": "https://anybot.test/problems/problem_internal"})),
    ),
    params(
        ("Idempotency-Key" = String, Header, description = "client-generated idempotency key")
    )
)]
pub async fn disable_two_step(
    State(state): State<AppState>,
    caller: Caller,
    request: axum::extract::Request,
) -> Result<Response, CallerDenied> {
    let key = idempotency_key(&request)?;
    let scope = caller_scope(&request, &caller.account);
    let (body, _raw) = read_json::<TwoStepCodeRequest>(request).await?;
    let pool = pool_of(&state)?;
    if let Some(replay) = guard_replay(pool, &key, EMPTY_FINGERPRINT, &scope).await? {
        return Ok(replay);
    }
    let enabled = two_step_enabled(pool, &caller.account)
        .await
        .map_err(internal_sqlx)?;
    if enabled {
        let keys = secrets_of(&state)?;
        let gate = CodeGate {
            state: &state,
            pool,
            keys: &keys,
            caller: &caller,
            code: body.code.as_deref(),
            slot: SecretSlot::Active,
        };
        require_code(&gate).await?;
        clear_two_step(pool, &caller.account)
            .await
            .map_err(internal_sqlx)?;
    }
    let ok = value_of(&ActionAck { ok: true })?;
    record(
        pool,
        &state,
        &key,
        EMPTY_FINGERPRINT,
        &scope,
        StatusCode::OK,
        &ok,
    )
    .await
}
