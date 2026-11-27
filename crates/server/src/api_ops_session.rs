use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;

use anybot_protocol::{ActionAck, ProblemJson};
use anybot_store_server::sessions;

use crate::api_ops::{ack_value, guard_replay, record};
use crate::app::AppState;
use crate::caller::CredentialKind;
use crate::caller::{Caller, CallerDenied};
use crate::req_write::{caller_scope, idempotency_key, internal_sqlx, pool_of, read_json};

#[utoipa::path(
    post,
    path = "/v1/account/logout",
    operation_id = "post_account_logout",
    summary = "Close the calling session",
    description = "Requires the session bearer key and the Idempotency-Key header.",
    tag = "account",
    security(("bearer" = [])),
    request_body(content = ActionAck, example = json!({"ok": true})),
    responses(
        (status = 200, description = "Session closed",
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
pub async fn logout(
    State(state): State<AppState>,
    caller: Caller,
    request: axum::extract::Request,
) -> Result<Response, CallerDenied> {
    let key = idempotency_key(&request)?;
    let scope = caller_scope(&request, &caller.account);
    let (body, raw) = read_json::<ActionAck>(request).await?;
    let pool = pool_of(&state)?;
    if let Some(replay) = guard_replay(pool, &key, &raw, &scope).await? {
        return Ok(replay);
    }
    if caller.credential == CredentialKind::Computer {
        crate::computers_service::remove(&state, pool, &caller.account, &caller.device_id).await?;
    } else {
        sessions::delete_sessions(pool, &[&caller.session_id])
            .await
            .map_err(internal_sqlx)?;
    }
    crate::sse_revoke::revoke_sse(&state, &caller.account);
    let value = ack_value(&body)?;
    let answer = record(pool, &state, &key, &raw, &scope, StatusCode::OK, &value).await?;
    Ok(clear_cookie(answer, &state, &caller).await)
}

async fn clear_cookie(answer: Response, state: &AppState, caller: &Caller) -> Response {
    if caller.credential != CredentialKind::Cookie {
        return answer;
    }
    let view = crate::auth_csrf::credential_view(state, None).await;
    let Ok(value) = axum::http::HeaderValue::from_str(&crate::auth_credential::cookie_clear(&view))
    else {
        return answer;
    };
    let mut answer = answer;
    answer
        .headers_mut()
        .append(axum::http::header::SET_COOKIE, value);
    answer
}

#[utoipa::path(
    post,
    path = "/v1/account/logout-all",
    operation_id = "post_account_logout_all",
    summary = "Close every phone and browser session of the account",
    description = "Requires the session bearer key and the Idempotency-Key header.",
    tag = "account",
    security(("bearer" = [])),
    request_body(content = ActionAck, example = json!({"ok": true})),
    responses(
        (status = 200, description = "All sessions closed",
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
pub async fn logout_all(
    State(state): State<AppState>,
    caller: Caller,
    request: axum::extract::Request,
) -> Result<Response, CallerDenied> {
    let key = idempotency_key(&request)?;
    let scope = caller_scope(&request, &caller.account);
    let (body, raw) = read_json::<ActionAck>(request).await?;
    let pool = pool_of(&state)?;
    if let Some(replay) = guard_replay(pool, &key, &raw, &scope).await? {
        return Ok(replay);
    }
    sessions::delete_sessions_for_account(pool, &caller.account, None)
        .await
        .map_err(internal_sqlx)?;
    crate::sse_revoke::revoke_sse(&state, &caller.account);
    let value = ack_value(&body)?;
    record(pool, &state, &key, &raw, &scope, StatusCode::OK, &value).await
}
