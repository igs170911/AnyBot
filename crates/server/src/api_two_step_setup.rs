use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;

use anybot_protocol::{ProblemJson, TwoStepCodeRequest, TwoStepSetup};
use anybot_store_server::two_step::{PendingSetup, store_pending, two_step_enabled};

use crate::app::AppState;
use crate::caller::{Caller, CallerDenied};
use crate::req_write::{
    Outcome, caller_scope, idempotency_key, internal_sqlx, pool_of, read_json, record_outcome,
    value_of,
};
use crate::two_step_check::{now_ms, secrets_of, unreadable};
use crate::two_step_gate::{CodeGate, EMPTY_FINGERPRINT, SecretSlot, fresh_setup, require_code};

#[utoipa::path(
    post,
    path = "/v1/account/two-step/setup",
    operation_id = "post_account_two_step_setup",
    summary = "Start setting up two-step verification",
    description = "Returns a new authenticator secret and 8 recovery codes, shown only this once. Nothing changes until the setup is confirmed. When two-step is already on, the current code is required. Requires the Idempotency-Key header.",
    tag = "account",
    security(("bearer" = [])),
    request_body(content = TwoStepCodeRequest, example = json!({"code": "123456"})),
    responses(
        (status = 200, description = "Setup started",
            content_type = "application/json",
            body = TwoStepSetup,
            example = json!({"otpauth_uri": "otpauth://totp/AnyBot:alice?secret=GEZDGNBVGY3TQOJQ&issuer=AnyBot&period=30&digits=6", "secret": "GEZDGNBVGY3TQOJQ", "recovery_codes": ["0a1b2c3d4e", "5f6a7b8c9d"]})),
        (status = 401, description = "Current code missing or wrong",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"code": "account-two-step-setup-code-wrong", "params": {}, "status": 401, "title": "The verification code is wrong: enter the current six digits from your authenticator app.", "type": "https://anybot.test/problems/account_two_step_setup_code_wrong"})),
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
pub async fn setup_two_step(
    State(state): State<AppState>,
    caller: Caller,
    request: axum::extract::Request,
) -> Result<Response, CallerDenied> {
    let key = idempotency_key(&request)?;
    let scope = caller_scope(&request, &caller.account);
    let (body, _raw) = read_json::<TwoStepCodeRequest>(request).await?;
    let pool = pool_of(&state)?;
    let keys = secrets_of(&state)?;
    let enabled = two_step_enabled(pool, &caller.account)
        .await
        .map_err(internal_sqlx)?;
    if enabled {
        let gate = CodeGate {
            state: &state,
            pool,
            keys: &keys,
            caller: &caller,
            code: body.code.as_deref(),
            slot: SecretSlot::Active,
        };
        require_code(&gate).await?;
    }
    let fresh = fresh_setup(&caller.account)?;
    let pending = PendingSetup {
        account: &caller.account,
        secret: &fresh.secret,
        recovery_codes: &fresh.recovery_codes,
        at_ms: now_ms(&state)?,
    };
    store_pending(pool, &keys, &pending)
        .await
        .map_err(|err| unreadable(&caller.account, &err))?;
    let value = value_of(&fresh.shown)?;
    record_outcome(
        pool,
        &state,
        &Outcome {
            key: &key,
            raw: EMPTY_FINGERPRINT,
            scope: &scope,
            status: StatusCode::OK,
            value: &value,
            marker: true,
        },
    )
    .await
}
