use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;

use anybot_messages::MessageCode;
use anybot_protocol::{ActionAck, ProblemJson};
use anybot_store_server::{accounts, sessions};

use crate::api_ops::guard_replay;
use crate::api_ops::record;
use crate::app::AppState;
use crate::caller::{Caller, CallerDenied};
use crate::req_write::{denied, idempotency_key, internal_sqlx, pool_of, read_json, value_of};

use crate::api_admin_reset::{admin_only, normalized_target};
use anybot_store_server::config;
#[utoipa::path(
    post,
    path = "/v1/admin/accounts/{account}/disable",
    operation_id = "post_admin_account_disable",
    summary = "Disable an account",
    description = "Admin only; requires the Idempotency-Key header. The caller account cannot be disabled.",
    tag = "admin",
    security(("bearer" = [])),
    request_body(content = ActionAck, example = json!({"ok": true})),
    responses(
        (status = 200, description = "Account disabled",
            content_type = "application/json",
            body = ActionAck,
            example = json!({"ok": true})),
        (status = 400, description = "Missing idempotency key",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_idempotency_missing", "title": "The Idempotency-Key header is required.", "status": 400, "code": "account-idempotency-missing", "params": {}})),
        (status = 401, description = "Session rejected",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_login_credentials", "title": "This session is no longer valid.", "status": 401, "code": "account-login-credentials", "params": {}})),
        (status = 403, description = "Not an admin",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_admin_only", "title": "Only the server admin can do this.", "status": 403, "code": "account-admin-only", "params": {}})),
        (status = 403, description = "Self disable",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_admin_self_disable", "title": "The server admin cannot disable this own account.", "status": 403, "code": "account-admin-self-disable", "params": {}})),
        (status = 404, description = "No such account",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_login_credentials", "title": "The account or password is wrong.", "status": 404, "code": "account-login-credentials", "params": {}})),
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
        ("account" = String, Path, description = "account name"),
        ("Idempotency-Key" = String, Header, description = "client-generated idempotency key")
    )
)]
pub async fn disable_account(
    State(state): State<AppState>,
    caller: Caller,
    request: axum::extract::Request,
) -> Result<Response, CallerDenied> {
    admin_only(&caller)?;
    let key = idempotency_key(&request)?;
    let scope = crate::req_write::caller_scope(&request, &caller.account);
    let target = normalized_target(&request)?;
    let (body, raw) = read_json::<ActionAck>(request).await?;
    let pool = pool_of(&state)?;
    if let Some(replay) = guard_replay(pool, &key, &raw, &scope).await? {
        return Ok(replay);
    }
    if target == caller.account {
        return Err(denied(
            StatusCode::FORBIDDEN,
            MessageCode::AccountAdminSelfDisable,
        ));
    }
    let found = accounts::find_account(pool, &target)
        .await
        .map_err(internal_sqlx)?;
    found.ok_or(denied(
        StatusCode::NOT_FOUND,
        MessageCode::AccountLoginCredentials,
    ))?;
    accounts::set_disabled(pool, &target, true)
        .await
        .map_err(internal_sqlx)?;
    sessions::delete_sessions_for_account(pool, &target, None)
        .await
        .map_err(internal_sqlx)?;
    let value = value_of(&body)?;
    record(pool, &state, &key, &raw, &scope, StatusCode::OK, &value).await
}

#[utoipa::path(
    post,
    path = "/v1/admin/accounts/{account}/enable",
    operation_id = "post_admin_account_enable",
    summary = "Enable an account",
    description = "Admin only; requires the Idempotency-Key header.",
    tag = "admin",
    security(("bearer" = [])),
    request_body(content = ActionAck, example = json!({"ok": true})),
    responses(
        (status = 200, description = "Account enabled",
            content_type = "application/json",
            body = ActionAck,
            example = json!({"ok": true})),
        (status = 400, description = "Missing idempotency key",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_idempotency_missing", "title": "The Idempotency-Key header is required.", "status": 400, "code": "account-idempotency-missing", "params": {}})),
        (status = 401, description = "Session rejected",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_login_credentials", "title": "This session is no longer valid.", "status": 401, "code": "account-login-credentials", "params": {}})),
        (status = 403, description = "Not an admin",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_admin_only", "title": "Only the server admin can do this.", "status": 403, "code": "account-admin-only", "params": {}})),
        (status = 404, description = "No such account",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_login_credentials", "title": "The account or password is wrong.", "status": 404, "code": "account-login-credentials", "params": {}})),
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
        ("account" = String, Path, description = "account name"),
        ("Idempotency-Key" = String, Header, description = "client-generated idempotency key")
    )
)]
pub async fn enable_account(
    State(state): State<AppState>,
    caller: Caller,
    request: axum::extract::Request,
) -> Result<Response, CallerDenied> {
    admin_only(&caller)?;
    let key = idempotency_key(&request)?;
    let scope = crate::req_write::caller_scope(&request, &caller.account);
    let target = normalized_target(&request)?;
    let (body, raw) = read_json::<ActionAck>(request).await?;
    let pool = pool_of(&state)?;
    if let Some(replay) = guard_replay(pool, &key, &raw, &scope).await? {
        return Ok(replay);
    }
    let found = accounts::find_account(pool, &target)
        .await
        .map_err(internal_sqlx)?;
    found.ok_or(denied(
        StatusCode::NOT_FOUND,
        MessageCode::AccountLoginCredentials,
    ))?;
    accounts::set_disabled(pool, &target, false)
        .await
        .map_err(internal_sqlx)?;
    let value = value_of(&body)?;
    record(pool, &state, &key, &raw, &scope, StatusCode::OK, &value).await
}

pub async fn read_invite(pool: &anybot_store_server::Pool) -> Result<Option<String>, CallerDenied> {
    let rows = config::read_config_rows(pool)
        .await
        .map_err(internal_sqlx)?;
    Ok(rows
        .into_iter()
        .find(|(key, _)| key == crate::config::KEY_INVITE_CODE)
        .map(|(_, value)| value))
}
