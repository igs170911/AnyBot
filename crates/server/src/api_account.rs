use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;

use anybot_domain::account_name::normalize_account_name;
use anybot_foundation::Secret;
use anybot_messages::MessageCode;
use anybot_protocol::{LoginRequest, LoginSession, ProblemJson, TwoStepChallenge};
use anybot_store_server::{Pool, accounts, two_step};

use crate::api_login_flow::{credential_is_cookie, slow_fail, verify_password};
use crate::api_login_issue::{SessionIssue, issue_session};
use crate::api_register::status_from_i32;
use crate::app::AppState;
use crate::caller::CallerDenied;
use crate::idem::{DID_IT_BODY, find_recorded, replay_value, request_scope, safe_digest};
use crate::limits::Limiter;
use crate::problems;
use crate::req_write::{
    Outcome, denied, internal_detail, internal_sqlx, internal_text, pool_of, record_outcome,
};
use axum::http::Method;

#[utoipa::path(
    post,
    path = "/v1/account/login",
    operation_id = "post_account_login",
    summary = "Log in and open a session",
    description = "Requires the Idempotency-Key header; replaying the same key opens a new session.",
    tag = "account",
    security(()),
    request_body(content = LoginRequest, example = json!({"account": "alice", "password": "hunter22", "label": "phone", "credential": "bearer"})),
    responses(
        (status = 200, description = "Session opened",
            content_type = "application/json",
            body = LoginSession,
            example = json!({"bearer_key": "a1b2c3d4e5f6a1b2", "account": "alice", "device_id": "d1"})),
        (status = 202, description = "Password accepted; the two-step code is required next",
            content_type = "application/json",
            body = TwoStepChallenge,
            example = json!({"challenge": "9f2c4e7a1b3d5f60", "account": "alice"})),
        (status = 400, description = "Missing idempotency key",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"code": "account-idempotency-missing", "params": {}, "status": 400, "title": "The Idempotency-Key header is required.", "type": "https://anybot.test/problems/account_idempotency_missing"})),
        (status = 401, description = "Wrong credentials",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"code": "account-login-credentials", "params": {}, "status": 401, "title": "The account or password is wrong.", "type": "https://anybot.test/problems/account_login_credentials"})),
        (status = 403, description = "Disabled account",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"code": "account-login-disabled", "params": {}, "status": 403, "title": "This account is disabled by the server admin.", "type": "https://anybot.test/problems/account_login_disabled"})),
        (status = 409, description = "Idempotency key conflict",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"code": "account-idempotency-conflict", "params": {}, "status": 409, "title": "This request id was already used with a different body.", "type": "https://anybot.test/problems/account_idempotency_conflict"})),
        (status = 429, description = "Login rate limit reached",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"code": "account-login-rate", "params": {}, "status": 429, "title": "Too many password failures; try again in 5 minutes.", "type": "https://anybot.test/problems/account_login_rate"})),
        (status = 500, description = "Internal error",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"code": "problem-internal", "params": {}, "status": 500, "title": "The server hit an internal error.", "type": "https://anybot.test/problems/problem_internal"})),
    ),
    params(
        ("Idempotency-Key" = String, Header, description = "client-generated idempotency key")
    )
)]
pub async fn login(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Result<Response, CallerDenied> {
    let (parts, body) = request.into_parts();
    let raw = axum::body::to_bytes(body, crate::req_write::MAX_WRITE_BODY)
        .await
        .map_err(|e| internal_detail(&e.to_string()))?
        .to_vec();
    let key = crate::req_write::idempotency_key_of(&parts.headers)?;
    let address = crate::req_write::parts_address(&parts, &state).await?;
    let path = parts.uri.path().to_owned();
    let origin = crate::api_login_issue::origin_of(&parts.headers);
    let body: LoginRequest = serde_json::from_slice(&raw)
        .map_err(|e| crate::req_write::denied_detail(StatusCode::BAD_REQUEST, &e.to_string()))?;
    let pool = pool_of(&state)?;
    let account = normalized_account(&body)?;
    let scope = request_scope(&Method::POST, &path, &account);
    let _guard = guard_idem(pool, &key, &raw, &scope).await?;
    let limiter = Limiter::new(pool, std::sync::Arc::clone(&state.clock));
    let attempt = limiter
        .begin_login(&address, &account)
        .await
        .map_err(internal_sqlx)?;
    let Some(id) = attempt else {
        slow_fail(&state).await;
        return Err(denied(
            StatusCode::TOO_MANY_REQUESTS,
            MessageCode::AccountLoginRate,
        ));
    };
    let cookie_mode = credential_is_cookie(&body)?;
    let computer = crate::api_login_flow::credential_is_computer(&body);
    let run = LoginRun {
        state: &state,
        pool,
        limiter: &limiter,
        attempt: id,
        account: &account,
        body: &body,
        key: &key,
        raw: &raw,
        scope: &scope,
        origin,
        cookie_mode,
        computer,
    };
    run_login(&run).await
}

struct LoginRun<'a> {
    state: &'a AppState,
    pool: &'a Pool,
    limiter: &'a Limiter,
    attempt: i64,
    account: &'a str,
    body: &'a LoginRequest,
    key: &'a str,
    raw: &'a [u8],
    scope: &'a str,
    origin: Option<String>,
    cookie_mode: bool,
    computer: bool,
}

async fn run_login(run: &LoginRun<'_>) -> Result<Response, CallerDenied> {
    let row = accounts::find_account(run.pool, run.account)
        .await
        .map_err(internal_sqlx)?;
    let verified = verify_password(
        run.state,
        row.as_ref(),
        &Secret::from_untrusted(run.body.password.clone()),
    )
    .await?;
    if !verified || row.is_none() {
        return rejected(run).await;
    }
    let stored = row.ok_or(internal_detail("missing account row"))?;
    if stored.disabled {
        return disabled(run).await;
    }
    accepted(run, run.account).await
}

fn normalized_account(body: &LoginRequest) -> Result<String, CallerDenied> {
    normalize_account_name(&body.account).map_err(|_| {
        denied(
            StatusCode::UNAUTHORIZED,
            MessageCode::AccountLoginCredentials,
        )
    })
}

async fn rejected(run: &LoginRun<'_>) -> Result<Response, CallerDenied> {
    slow_fail(run.state).await;
    let value = serde_json::to_value(problems::auth_problem(
        StatusCode::UNAUTHORIZED.as_u16(),
        MessageCode::AccountLoginCredentials,
    ))
    .map_err(internal_text)?;
    let answer = record_outcome(
        run.pool,
        run.state,
        &Outcome {
            key: run.key,
            raw: run.raw,
            scope: run.scope,
            status: StatusCode::UNAUTHORIZED,
            value: &value,
            marker: false,
        },
    )
    .await?;
    drop(answer);
    Err(denied(
        StatusCode::UNAUTHORIZED,
        MessageCode::AccountLoginCredentials,
    ))
}

async fn disabled(run: &LoginRun<'_>) -> Result<Response, CallerDenied> {
    let value = serde_json::to_value(problems::auth_problem(
        StatusCode::FORBIDDEN.as_u16(),
        MessageCode::AccountLoginDisabled,
    ))
    .map_err(internal_text)?;
    record_outcome(
        run.pool,
        run.state,
        &Outcome {
            key: run.key,
            raw: run.raw,
            scope: run.scope,
            status: StatusCode::FORBIDDEN,
            value: &value,
            marker: false,
        },
    )
    .await?;
    Err(denied(
        StatusCode::FORBIDDEN,
        MessageCode::AccountLoginDisabled,
    ))
}

async fn accepted(run: &LoginRun<'_>, account: &str) -> Result<Response, CallerDenied> {
    run.limiter
        .cancel_login(run.attempt)
        .await
        .map_err(internal_sqlx)?;
    let issue = SessionIssue {
        state: run.state,
        pool: run.pool,
        account,
        label: &run.body.label,
        cookie_mode: run.cookie_mode,
        computer: run.computer,
        used_recovery: false,
        origin: run.origin.as_deref(),
        key: run.key,
        raw: run.raw,
        scope: run.scope,
    };
    let enabled = two_step::two_step_enabled(run.pool, account)
        .await
        .map_err(internal_sqlx)?;
    if enabled {
        return crate::two_step_challenge::open_challenge(&issue).await;
    }
    issue_session(&issue).await
}

pub enum Guard {
    Fresh,
    Replay(StatusCode, serde_json::Value, bool),
}

pub async fn guard_idem(
    pool: &Pool,
    key: &str,
    raw: &[u8],
    scope: &str,
) -> Result<Guard, CallerDenied> {
    let stored = find_recorded(pool, key, scope)
        .await
        .map_err(internal_sqlx)?;
    match stored {
        None => Ok(Guard::Fresh),
        Some(record) if record.request_hash == safe_digest(raw)? => {
            let marker = record.body == DID_IT_BODY;
            let value = replay_value(&record.body)?;
            Ok(Guard::Replay(
                status_from_i32(record.status_code)?,
                value,
                marker,
            ))
        }
        Some(_) => Err(denied(
            StatusCode::CONFLICT,
            MessageCode::AccountIdempotencyConflict,
        )),
    }
}
