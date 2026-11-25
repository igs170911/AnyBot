use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;

use anybot_messages::MessageCode;
use anybot_protocol::{ActionAck, InviteResponse, ProblemJson, PublicUrlRequest};
use anybot_store_server::config;

use crate::api_ops::guard_replay;
use crate::api_ops::record;
use crate::app::AppState;
use crate::caller::{Caller, CallerDenied};
use crate::invite::draw_invite;
use crate::req_write::{
    denied, idempotency_key, internal_sqlx, internal_text, json_response, pool_of, read_json,
    value_of,
};

#[utoipa::path(
    get,
    path = "/v1/admin/invite",
    operation_id = "get_admin_invite",
    summary = "Read the current invite code",
    description = "Admin only.",
    tag = "admin",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Current invite code",
            content_type = "application/json",
            body = InviteResponse,
            example = json!({"invite": "2kqj-7mnp"})),
        (status = 401, description = "Session rejected",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_login_credentials", "title": "This session is no longer valid.", "status": 401, "code": "account-login-credentials", "params": {}})),
        (status = 403, description = "Not an admin",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_admin_only", "title": "Only the server admin can do this.", "status": 403, "code": "account-admin-only", "params": {}})),
        (status = 404, description = "No invite code set",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_invite_invalid", "title": "The invite code is not valid.", "status": 404, "code": "account-invite-invalid", "params": {}})),
        (status = 500, description = "Internal error",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/problem_internal", "title": "The server hit an internal error.", "status": 500, "code": "problem-internal", "params": {}})),
    )
)]
pub async fn invite(
    State(state): State<AppState>,
    caller: Caller,
) -> Result<Response, CallerDenied> {
    admin_only(&caller)?;
    let pool = pool_of(&state)?;
    let value = read_invite(pool).await?.ok_or(denied(
        StatusCode::NOT_FOUND,
        MessageCode::AccountInviteInvalid,
    ))?;
    Ok(json_response(
        StatusCode::OK,
        value_of(&InviteResponse { invite: value })?,
    ))
}

#[utoipa::path(
    post,
    path = "/v1/admin/invite/rotate",
    operation_id = "post_admin_invite_rotate",
    summary = "Generate a new invite code",
    description = "Admin only; requires the Idempotency-Key header.",
    tag = "admin",
    security(("bearer" = [])),
    request_body(content = ActionAck, example = json!({"ok": true})),
    responses(
        (status = 200, description = "New invite code",
            content_type = "application/json",
            body = InviteResponse,
            example = json!({"invite": "2kqj-7mnp"})),
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
pub async fn rotate_invite(
    State(state): State<AppState>,
    caller: Caller,
    request: axum::extract::Request,
) -> Result<Response, CallerDenied> {
    admin_only(&caller)?;
    let key = idempotency_key(&request)?;
    let scope = crate::req_write::caller_scope(&request, &caller.account);
    let (_body, raw) = read_json::<ActionAck>(request).await?;
    let pool = pool_of(&state)?;
    let _guard = crate::api_account::guard_idem(pool, &key, &raw, &scope).await?;
    let fresh = draw_invite().map_err(internal_text)?;
    config::write_config_value(pool, crate::config::KEY_INVITE_CODE, &fresh)
        .await
        .map_err(internal_sqlx)?;
    let value = value_of(&InviteResponse { invite: fresh })?;
    crate::req_write::record_outcome(
        pool,
        &state,
        &crate::req_write::Outcome {
            key: &key,
            raw: &raw,
            scope: &scope,
            status: StatusCode::OK,
            value: &value,
            marker: true,
        },
    )
    .await
}

#[utoipa::path(
    post,
    path = "/v1/admin/public-url",
    operation_id = "post_admin_public_url",
    summary = "Set the public URL of this server",
    description = "Admin only; requires the Idempotency-Key header.",
    tag = "admin",
    security(("bearer" = [])),
    request_body(content = PublicUrlRequest, example = json!({"public_url": "https://bot.example.test"})),
    responses(
        (status = 200, description = "URL stored",
            content_type = "application/json",
            body = ActionAck,
            example = json!({"ok": true})),
        (status = 400, description = "Missing idempotency key",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_idempotency_missing", "title": "The Idempotency-Key header is required.", "status": 400, "code": "account-idempotency-missing", "params": {}})),
        (status = 400, description = "Bad URL",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/problem_bad_request", "title": "The public URL is not a valid http or https URL.", "status": 400, "code": "problem-bad-request", "params": {}})),
        (status = 401, description = "Session rejected",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_login_credentials", "title": "This session is no longer valid.", "status": 401, "code": "account-login-credentials", "params": {}})),
        (status = 403, description = "Not an admin",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_admin_only", "title": "Only the server admin can do this.", "status": 403, "code": "account-admin-only", "params": {}})),
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
pub async fn set_public_url(
    State(state): State<AppState>,
    caller: Caller,
    request: axum::extract::Request,
) -> Result<Response, CallerDenied> {
    admin_only(&caller)?;
    let key = idempotency_key(&request)?;
    let scope = crate::req_write::caller_scope(&request, &caller.account);
    let (body, raw) = read_json::<PublicUrlRequest>(request).await?;
    let pool = pool_of(&state)?;
    if let Some(replay) = guard_replay(pool, &key, &raw, &scope).await? {
        return Ok(replay);
    }
    let parsed = url::Url::parse(&body.public_url)
        .map_err(|_| denied(StatusCode::BAD_REQUEST, MessageCode::ProblemBadRequest))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(denied(
            StatusCode::BAD_REQUEST,
            MessageCode::ProblemBadRequest,
        ));
    }
    config::write_config_value(pool, crate::config::KEY_PUBLIC_URL, &body.public_url)
        .await
        .map_err(internal_sqlx)?;
    let value = value_of(&ActionAck { ok: true })?;
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

fn admin_only(caller: &Caller) -> Result<(), CallerDenied> {
    if caller.admin {
        return Ok(());
    }
    Err(denied(StatusCode::FORBIDDEN, MessageCode::AccountAdminOnly))
}
