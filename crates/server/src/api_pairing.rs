use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;

use anybot_domain::computer_actions::AccountAction;
use anybot_foundation::Secret;
use anybot_messages::MessageCode;
use anybot_protocol::{
    ActionAck, PairingCodeReply, PairingCodeRequest, PasswordReenterRequest, ProblemJson,
};
use anybot_store_server::accounts;

use crate::api_login_flow::{slow_fail, verify_password};
use crate::app::AppState;
use crate::caller::Caller;
use crate::computers_action::{Step, finish, prepare, require_allowed};
use crate::limits::Limiter;
use crate::pairing_issue::issue_code;
use crate::reauth_state::{record_password_entry, require_fresh_password};
use crate::req_write::{
    CallerDenied, denied, internal_detail, internal_sqlx, json_response, pool_of, read_json,
    value_of,
};

#[utoipa::path(
    post,
    path = "/v1/computers/pairing-code",
    operation_id = "post_computers_pairing_code",
    summary = "Make a pairing code for a new computer",
    description = "Six digits, valid for 10 minutes, one use. The password must have been entered in the last 15 minutes. A retry makes a new code: the code itself is never stored.",
    tag = "computers",
    security(("bearer" = [])),
    request_body(content = PairingCodeRequest, example = json!({})),
    responses(
        (status = 200, description = "Done",
            content_type = "application/json",
            body = PairingCodeReply,
            example = json!({"code": "048213", "expires_at_ms": 1_700_000_600_000_i64})),
        (status = 401, description = "Credential rejected",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_login_credentials", "title": "This session is no longer valid.", "status": 401, "code": "account-login-credentials", "params": {}})),
        (status = 403, description = "A paired computer cannot do this",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/computer_paired_forbidden", "title": "A computer added with a pairing code cannot do this", "status": 403, "code": "computer-paired-forbidden", "params": {}})),
        (status = 403, description = "The password must be entered again first",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_reauth_required", "title": "Enter your password again first", "status": 403, "code": "account-reauth-required", "params": {}})),
        (status = 429, description = "Too many pairing codes are waiting",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/computers_pair_full", "title": "Too many computers are waiting to pair: try again later", "status": 429, "code": "computers-pair-full", "params": {}})),
        (status = 500, description = "Internal error",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/problem_internal", "title": "The server hit an internal error.", "status": 500, "code": "problem-internal", "params": {}})),
    ),
)]
pub async fn create_pairing_code(
    State(state): State<AppState>,
    caller: Caller,
    request: axum::extract::Request,
) -> Result<Response, CallerDenied> {
    require_allowed(&caller, AccountAction::GeneratePairingCode)?;
    let (_empty, _raw) = read_json::<PairingCodeRequest>(request).await?;
    let pool = pool_of(&state)?;
    require_fresh_password(&state, pool, &caller).await?;
    let keys = state
        .secrets
        .as_ref()
        .ok_or_else(|| internal_detail("secret keys are not loaded"))?;
    let now = state.clock.now_ms().millis();
    let reply: PairingCodeReply = issue_code(keys, pool, &caller.account, now).await?;
    Ok(json_response(StatusCode::OK, value_of(&reply)?))
}

#[utoipa::path(
    post,
    path = "/v1/account/reauth",
    operation_id = "post_account_reauth",
    summary = "Enter the password again",
    description = "Starts a fresh 15 minutes for actions that need a recent password.",
    tag = "account",
    security(("bearer" = [])),
    request_body(content = PasswordReenterRequest, example = json!({"password": "longenough"})),
    responses(
        (status = 200, description = "Done",
            content_type = "application/json",
            body = ActionAck,
            example = json!({"ok": true})),
        (status = 400, description = "Missing idempotency key",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_idempotency_missing", "title": "The Idempotency-Key header is required.", "status": 400, "code": "account-idempotency-missing", "params": {}})),
        (status = 401, description = "Credential rejected",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_login_credentials", "title": "This session is no longer valid.", "status": 401, "code": "account-login-credentials", "params": {}})),
        (status = 403, description = "The password is wrong",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_reauth_password_wrong", "title": "The password is wrong", "status": 403, "code": "account-reauth-password-wrong", "params": {}})),
        (status = 429, description = "Too many password attempts",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_password_rate", "title": "Too many password change attempts; try again in 5 minutes.", "status": 429, "code": "account-password-rate", "params": {}})),
        (status = 409, description = "Idempotency key conflict",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_idempotency_conflict", "title": "This request id was already used with a different body.", "status": 409, "code": "account-idempotency-conflict", "params": {}})),
        (status = 500, description = "Internal error",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/problem_internal", "title": "The server hit an internal error.", "status": 500, "code": "problem-internal", "params": {}})),
    ),
    params(
        ("Idempotency-Key" = String, Header, description = "client-generated idempotency key")
    )
)]
pub async fn reauth_password(
    State(state): State<AppState>,
    caller: Caller,
    request: axum::extract::Request,
) -> Result<Response, CallerDenied> {
    let prepared = match prepare::<PasswordReenterRequest>(&state, &caller, request).await? {
        Step::Replay(response) => return Ok(response),
        Step::Run(prepared) => prepared,
    };
    let pool = pool_of(&state)?;
    let limiter = Limiter::new(pool, std::sync::Arc::clone(&state.clock));
    let admitted = limiter
        .begin_password(&caller.address, &caller.account)
        .await
        .map_err(internal_sqlx)?;
    if !admitted {
        return Err(denied(
            StatusCode::TOO_MANY_REQUESTS,
            MessageCode::AccountPasswordRate,
        ));
    }
    let row = accounts::find_account(pool, &caller.account)
        .await
        .map_err(internal_sqlx)?;
    let secret = Secret::from_untrusted(prepared.body.password.clone());
    if !verify_password(&state, row.as_ref(), &secret).await? {
        slow_fail(&state).await;
        return Err(denied(
            StatusCode::FORBIDDEN,
            MessageCode::AccountReauthPasswordWrong,
        ));
    }
    record_password_entry(&state, pool, &caller).await?;
    let value = value_of(&ActionAck { ok: true })?;
    finish(&state, &prepared, StatusCode::OK, &value).await
}
