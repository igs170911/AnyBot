use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;

use anybot_domain::account_name::normalize_account_name;
use anybot_foundation::Secret;
use anybot_messages::MessageCode;
use anybot_protocol::{ActionAck, ProblemJson, ResetPasswordRequest};
use anybot_store_server::{accounts, sessions};

use crate::api_ops::{guard_replay, record};
use crate::app::AppState;
use crate::caller::{Caller, CallerDenied};
use crate::req_write::{
    denied, idempotency_key, internal_sqlx, internal_text, pool_of, read_json, value_of,
};

pub(super) fn normalized_target(request: &axum::extract::Request) -> Result<String, CallerDenied> {
    let raw = crate::req_write::path_account(request)?;
    normalize_account_name(&raw)
        .map_err(|_| denied(StatusCode::NOT_FOUND, MessageCode::AccountLoginCredentials))
}
#[utoipa::path(
    post,
    path = "/v1/admin/accounts/{account}/reset-password",
    operation_id = "post_admin_account_reset_password",
    summary = "Reset an account password as admin",
    description = "Admin only; requires the Idempotency-Key header. Closes every session of the account.",
    tag = "admin",
    security(("bearer" = [])),
    request_body(content = ResetPasswordRequest, example = json!({"new": "sunny2233"})),
    responses(
        (status = 200, description = "Password reset",
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
        (status = 401, description = "Session rejected",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"code": "account-login-credentials", "params": {}, "status": 401, "title": "This session is no longer valid.", "type": "https://anybot.test/problems/account_login_credentials"})),
        (status = 403, description = "Not an admin",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"code": "account-admin-only", "params": {}, "status": 403, "title": "Only the server admin can do this.", "type": "https://anybot.test/problems/account_admin_only"})),
        (status = 404, description = "No such account",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"code": "account-exists-reset", "params": {}, "status": 404, "title": "This account already exists on the server; if you forgot the password, ask the server admin to reset it.", "type": "https://anybot.test/problems/account_exists_reset"})),
        (status = 409, description = "Idempotency key conflict",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"code": "account-idempotency-conflict", "params": {}, "status": 409, "title": "This request id was already used with a different body.", "type": "https://anybot.test/problems/account_idempotency_conflict"})),
        (status = 500, description = "Internal error",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"code": "problem-internal", "params": {}, "status": 500, "title": "The server hit an internal error.", "type": "https://anybot.test/problems/problem_internal"})),
    ),
    params(
        ("account" = String, Path, description = "account name"),
        ("Idempotency-Key" = String, Header, description = "client-generated idempotency key")
    )
)]
pub async fn reset_password(
    State(state): State<AppState>,
    caller: Caller,
    request: axum::extract::Request,
) -> Result<Response, CallerDenied> {
    admin_only(&caller)?;
    let key = idempotency_key(&request)?;
    let scope = crate::req_write::caller_scope(&request, &caller.account);
    let target = normalized_target(&request)?;
    let (body, raw) = read_json::<ResetPasswordRequest>(request).await?;
    let pool = pool_of(&state)?;
    if let Some(replay) = guard_replay(pool, &key, &raw, &scope).await? {
        return Ok(replay);
    }
    let value = apply_reset(&state, pool, &target, &body).await?;
    record(pool, &state, &key, &raw, &scope, StatusCode::OK, &value).await
}

async fn apply_reset(
    state: &AppState,
    pool: &anybot_store_server::Pool,
    target: &str,
    body: &ResetPasswordRequest,
) -> Result<serde_json::Value, CallerDenied> {
    let row = accounts::find_account(pool, target)
        .await
        .map_err(internal_sqlx)?;
    let row = row.ok_or(denied(
        StatusCode::NOT_FOUND,
        MessageCode::AccountLoginCredentials,
    ))?;
    if !anybot_domain::account_name::is_allowed_password(&body.new) {
        return Err(denied(
            StatusCode::BAD_REQUEST,
            MessageCode::AccountPasswordNewShort,
        ));
    }
    let hash = state
        .pass_hasher()
        .await
        .map_err(internal_text)?
        .hash(&Secret::from_untrusted(body.new.clone()))
        .await
        .map_err(internal_text)?;
    let at_ms = i64::try_from(state.clock.now_ms().millis()).map_err(internal_text)?;
    accounts::set_password(pool, &row.account, &hash, at_ms)
        .await
        .map_err(internal_sqlx)?;
    sessions::delete_sessions_for_account(pool, &row.account, None)
        .await
        .map_err(internal_sqlx)?;
    value_of(&ActionAck { ok: true })
}

pub(super) fn admin_only(caller: &Caller) -> Result<(), CallerDenied> {
    if caller.admin {
        return Ok(());
    }
    Err(denied(StatusCode::FORBIDDEN, MessageCode::AccountAdminOnly))
}
