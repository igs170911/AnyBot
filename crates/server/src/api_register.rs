use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;

use anybot_domain::account_name::{is_allowed_password, normalize_account_name};
use anybot_foundation::Secret;
use anybot_messages::MessageCode;
use anybot_protocol::{ProblemJson, RegisterAck, RegisterRequest};
use anybot_store_server::accounts;

use crate::app::AppState;
use crate::caller::CallerDenied;
use crate::limits::Limiter;

use crate::api_account::{Guard, guard_idem};
use crate::idem::request_scope;
use crate::req_write::{
    Outcome, denied, internal_sqlx, internal_text, json_response, pool_of, record_outcome,
};
use axum::http::Method;

pub(crate) fn status_from_i32(code: i32) -> Result<StatusCode, CallerDenied> {
    let raw = u16::try_from(code).map_err(internal_text)?;
    StatusCode::from_u16(raw).map_err(internal_text)
}

#[utoipa::path(
    post,
    path = "/v1/account/register",
    operation_id = "post_account_register",
    summary = "Create an account with an invite code",
    description = "Requires the Idempotency-Key header; the invite code must match the server setting.",
    tag = "account",
    security(()),
    request_body(content = RegisterRequest, example = json!({"account": "alice", "password": "hunter22", "invite": "abcd-efgh"})),
    responses(
        (status = 201, description = "Account created",
            content_type = "application/json",
            body = RegisterAck,
            example = json!({"created": true, "admin": true})),
        (status = 400, description = "Name rule failed",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_name_rule", "title": "The account name does not meet the rule.", "status": 400, "code": "account-name-rule", "params": {}})),
        (status = 400, description = "Password rule failed",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_password_rule", "title": "The password does not meet the rule.", "status": 400, "code": "account-password-rule", "params": {}})),
        (status = 400, description = "Missing idempotency key",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_idempotency_missing", "title": "The Idempotency-Key header is required.", "status": 400, "code": "account-idempotency-missing", "params": {}})),
        (status = 403, description = "Invite code rejected",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_invite_invalid", "title": "The invite code is not valid.", "status": 403, "code": "account-invite-invalid", "params": {}})),
        (status = 409, description = "Account exists",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_exists", "title": "This account is already taken.", "status": 409, "code": "account-exists", "params": {}})),
        (status = 409, description = "Idempotency key conflict",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_idempotency_conflict", "title": "This request id was already used with a different body.", "status": 409, "code": "account-idempotency-conflict", "params": {}})),
        (status = 429, description = "Registration rate limit reached",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_register_rate", "title": "Too many registrations from this address; try again in 5 minutes.", "status": 429, "code": "account-register-rate", "params": {}})),
        (status = 500, description = "Internal error",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/problem_internal", "title": "The server hit an internal error.", "status": 500, "code": "problem-internal", "params": {}})),
    ),
    params(
        ("Idempotency-Key" = String, Header, description = "client-generated idempotency key")
    )
)]

pub async fn register(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Result<Response, CallerDenied> {
    let (parts, body) = request.into_parts();
    let raw = axum::body::to_bytes(body, crate::req_write::MAX_WRITE_BODY)
        .await
        .map_err(|e| crate::req_write::internal_detail(&e.to_string()))?
        .to_vec();
    let key = crate::req_write::idempotency_key_of(&parts.headers)?;
    let address = crate::req_write::parts_address(&parts, &state).await?;
    let path = parts.uri.path().to_owned();
    let body: RegisterRequest = serde_json::from_slice(&raw)
        .map_err(|e| crate::req_write::denied_detail(StatusCode::BAD_REQUEST, &e.to_string()))?;
    let pool = pool_of(&state)?;
    let account = normalize_account_name(&body.account)
        .map_err(|_| denied(StatusCode::BAD_REQUEST, MessageCode::AccountNameRule))?;
    let scope = request_scope(&Method::POST, &path, &account);
    let guard = guard_idem(pool, &key, &raw, &scope).await?;
    let limiter = Limiter::new(pool, std::sync::Arc::clone(&state.clock));
    let started = limiter
        .begin_register(&address)
        .await
        .map_err(internal_sqlx)?;
    let Some(_id) = started else {
        return Err(denied(
            StatusCode::TOO_MANY_REQUESTS,
            MessageCode::AccountRegisterRate,
        ));
    };
    if let Guard::Replay(replay_status, replay_value, _did_only) = guard {
        return Ok(json_response(replay_status, replay_value));
    }
    let (status, value) = run_register(&state, &body, &account).await?;
    record_outcome(
        pool,
        &state,
        &Outcome {
            key: &key,
            raw: &raw,
            scope: &scope,
            status,
            value: &value,
            marker: false,
        },
    )
    .await
}

pub async fn run_register(
    state: &AppState,
    body: &RegisterRequest,
    account: &str,
) -> Result<(StatusCode, serde_json::Value), CallerDenied> {
    let pool = pool_of(state)?;
    if !is_allowed_password(&body.password) {
        return Err(denied(
            StatusCode::BAD_REQUEST,
            MessageCode::AccountPasswordRule,
        ));
    }
    let hash = state
        .pass_hasher()
        .await
        .map_err(internal_text)?
        .hash(&Secret::from_untrusted(body.password.clone()))
        .await
        .map_err(internal_text)?;
    let now_ms = i64::try_from(state.clock.now_ms().millis()).map_err(internal_text)?;
    let mut tx = pool.begin().await.map_err(internal_sqlx)?;
    let expected =
        anybot_store_server::config::read_config_value_tx(&mut tx, crate::config::KEY_INVITE_CODE)
            .await
            .map_err(internal_sqlx)?;
    let Some(expected) = expected else {
        tx.rollback().await.map_err(internal_sqlx)?;
        return Err(denied(
            StatusCode::FORBIDDEN,
            MessageCode::AccountInviteInvalid,
        ));
    };
    if expected != body.invite {
        tx.rollback().await.map_err(internal_sqlx)?;
        return Err(denied(
            StatusCode::FORBIDDEN,
            MessageCode::AccountInviteInvalid,
        ));
    }
    let inserted = accounts::insert_account(&mut tx, account, &hash, now_ms)
        .await
        .map_err(internal_sqlx)?;
    let Some(admin) = inserted else {
        tx.rollback().await.map_err(internal_sqlx)?;
        return Err(denied(StatusCode::CONFLICT, MessageCode::AccountExists));
    };
    tx.commit().await.map_err(internal_sqlx)?;
    let value = serde_json::to_value(RegisterAck {
        created: true,
        admin,
    })
    .map_err(internal_text)?;
    Ok((StatusCode::CREATED, value))
}
