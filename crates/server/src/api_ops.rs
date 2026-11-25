use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;

use anybot_domain::account_name::is_allowed_password;
use anybot_foundation::Secret;
use anybot_messages::MessageCode;
use anybot_protocol::{ActionAck, PasswordChangeRequest, ProblemJson};
use anybot_store_server::{accounts, sessions};

use crate::app::AppState;
use crate::caller::{Caller, CallerDenied};
use crate::idem::find_recorded;
use crate::limits::{KIND_PASSWORD_CHANGE, Limiter};
use crate::req_write::{
    Outcome, caller_scope, denied, idempotency_key, internal_sqlx, internal_text, json_response,
    pool_of, read_json, record_outcome, value_of,
};

#[utoipa::path(
    post,
    path = "/v1/account/password",
    operation_id = "post_account_password",
    summary = "Change the caller password",
    description = "Requires the session bearer key and the Idempotency-Key header; every other session is closed.",
    tag = "account",
    security(("bearer" = [])),
    request_body(content = PasswordChangeRequest, example = json!({"old": "hunter22", "new": "sunny2233"})),
    responses(
        (status = 200, description = "Password changed",
            content_type = "application/json",
            body = ActionAck,
            example = json!({"ok": true})),
        (status = 400, description = "Missing idempotency key",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"code": "account-idempotency-missing", "params": {}, "status": 400, "title": "The account name does not meet the rule.", "type": "https://anybot.test/problems/account_idempotency_missing"})),
        (status = 400, description = "New password too short",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"code": "account-password-new-short", "params": {}, "status": 400, "title": "The new password is too short.", "type": "https://anybot.test/problems/account_password_new_short"})),
        (status = 401, description = "Old password wrong",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"code": "account-password-old-wrong", "params": {}, "status": 401, "title": "The old password is wrong.", "type": "https://anybot.test/problems/account_password_old_wrong"})),
        (status = 401, description = "Session rejected",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"code": "account-login-credentials", "params": {}, "status": 401, "title": "This session is no longer valid.", "type": "https://anybot.test/problems/account_login_credentials"})),
        (status = 409, description = "Idempotency key conflict",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"code": "account-idempotency-conflict", "params": {}, "status": 409, "title": "This request id was already used with a different body.", "type": "https://anybot.test/problems/account_idempotency_conflict"})),
        (status = 429, description = "Password change rate limit reached",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"code": "account-password-rate", "params": {}, "status": 429, "title": "Too many password change attempts; try again in 5 minutes.", "type": "https://anybot.test/problems/account_password_rate"})),
        (status = 500, description = "Internal error",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"code": "problem-internal", "params": {}, "status": 500, "title": "The server hit an internal error.", "type": "https://anybot.test/problems/problem_internal"})),
    ),
    params(
        ("Idempotency-Key" = String, Header, description = "client-generated idempotency key")
    )
)]
pub async fn change_password(
    State(state): State<AppState>,
    caller: Caller,
    request: axum::extract::Request,
) -> Result<Response, CallerDenied> {
    let key = idempotency_key(&request)?;
    let scope = caller_scope(&request, &caller.account);
    let (body, raw) = read_json::<PasswordChangeRequest>(request).await?;
    let pool = pool_of(&state)?;
    let limiter = Limiter::new(pool, std::sync::Arc::clone(&state.clock));
    if let Some(replay) = guard_replay(pool, &key, &raw, &scope).await? {
        return Ok(replay);
    }
    let blocked = limiter
        .password_blocked(&caller.account)
        .await
        .map_err(internal_sqlx)?;
    limiter
        .record(KIND_PASSWORD_CHANGE, &caller.address, &caller.account)
        .await
        .map_err(internal_sqlx)?;
    if blocked {
        return Err(denied(
            StatusCode::TOO_MANY_REQUESTS,
            MessageCode::AccountPasswordRate,
        ));
    }
    if !is_allowed_password(&body.new) {
        return Err(denied(
            StatusCode::BAD_REQUEST,
            MessageCode::AccountPasswordNewShort,
        ));
    }
    let row = accounts::find_account(pool, &caller.account)
        .await
        .map_err(internal_sqlx)?;
    let row = row.ok_or(denied(
        StatusCode::UNAUTHORIZED,
        MessageCode::AccountLoginCredentials,
    ))?;
    let hasher = state.pass_hasher().await.map_err(internal_text)?;
    let old_ok = hasher
        .verify(
            &Secret::from_untrusted(body.old.clone()),
            &row.password_hash,
        )
        .await
        .map_err(internal_text)?;
    if !old_ok {
        return Err(denied(
            StatusCode::UNAUTHORIZED,
            MessageCode::AccountPasswordOldWrong,
        ));
    }
    apply_password(&state, pool, &caller, &body.new).await
}

async fn apply_password(
    state: &AppState,
    pool: &anybot_store_server::Pool,
    caller: &Caller,
    new_password: &str,
) -> Result<Response, CallerDenied> {
    let hash = state
        .pass_hasher()
        .await
        .map_err(internal_text)?
        .hash(&Secret::from_untrusted(new_password.to_owned()))
        .await
        .map_err(internal_text)?;
    let at_ms = i64::try_from(state.clock.now_ms().millis()).map_err(internal_text)?;
    accounts::set_password(pool, &caller.account, &hash, at_ms)
        .await
        .map_err(internal_sqlx)?;
    sessions::delete_sessions_for_account(pool, &caller.account, Some(&caller.session_id))
        .await
        .map_err(internal_sqlx)?;
    let ok = serde_json::to_value(ActionAck { ok: true }).map_err(internal_text)?;
    Ok(json_response(StatusCode::OK, ok))
}

pub(crate) fn ack_value(body: &ActionAck) -> Result<serde_json::Value, CallerDenied> {
    value_of(body)
}

pub async fn guard_replay(
    pool: &anybot_store_server::Pool,
    key: &str,
    raw: &[u8],
    scope: &str,
) -> Result<Option<Response>, CallerDenied> {
    let stored = find_recorded(pool, key, scope)
        .await
        .map_err(internal_sqlx)?;
    match stored {
        None => Ok(None),
        Some(record) if record.request_hash == crate::idem::safe_digest(raw) => {
            let raw = u16::try_from(record.status_code).map_err(internal_text)?;
            let status = StatusCode::from_u16(raw).map_err(internal_text)?;
            let value = serde_json::from_str(&record.body).map_err(internal_text)?;
            Ok(Some(json_response(status, value)))
        }
        Some(_) => Err(denied(
            StatusCode::CONFLICT,
            MessageCode::AccountIdempotencyConflict,
        )),
    }
}

pub async fn record(
    pool: &anybot_store_server::Pool,
    state: &AppState,
    key: &str,
    raw: &[u8],
    scope: &str,
    status: StatusCode,
    value: &serde_json::Value,
) -> Result<Response, CallerDenied> {
    record_outcome(
        pool,
        state,
        &Outcome {
            key,
            raw,
            scope,
            status,
            value,
            marker: false,
        },
    )
    .await
}
