use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;

use anybot_domain::session_term::{evict_indices, session_expired};
use anybot_foundation::Secret;
use anybot_messages::MessageCode;
use anybot_domain::account_name::normalize_account_name;
use anybot_protocol::{LoginRequest, LoginSession, ProblemJson};
use anybot_store_server::{Pool, accounts, sessions};

use crate::api_register::status_from_i32;
use crate::app::AppState;
use crate::caller::CallerDenied;
use crate::idem::{DID_IT_BODY, find_recorded, request_scope, safe_digest};
use crate::limits::Limiter;
use crate::pass_hash::key_digest;
use crate::problems;
use crate::req_write::{
    denied, idempotency_key, internal_sqlx, internal_text, pool_of,
    read_json, record_outcome, request_address, path_of, value_of,
};
use axum::http::Method;

const LOGIN_DELAY_MS: u64 = 500;

#[utoipa::path(
    post,
    path = "/v1/account/login",
    operation_id = "post_account_login",
    summary = "Log in and open a session",
    description = "Requires the Idempotency-Key header; replaying the same key opens a new session.",
    tag = "account",
    security(()),
    request_body(content = LoginRequest, example = json!({"account": "alice", "password": "hunter22", "label": "phone"})),
    responses(
        (status = 200, description = "Session opened",
            content_type = "application/json",
            body = LoginSession,
            example = json!({"session_key": "a1b2c3d4e5f6a1b2", "account": "alice"})),
        (status = 400, description = "Missing idempotency key",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"code": "account-idempotency-missing", "params": {}, "status": 400, "title": "The Idempotency-Key header is required.", "type": "https://anybot.test/problems/account_idempotency_missing"})),
        (status = 401, description = "Wrong credentials",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"code": "account-login-credentials", "params": {}, "status": 401, "title": "The account or password is wrong.", "type": "https://anybot.test/problems/account_login_credentials"})),
        (status = 401, description = "Disabled account",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"code": "account-login-disabled", "params": {}, "status": 401, "title": "This account is disabled by the server admin.", "type": "https://anybot.test/problems/account_login_disabled"})),
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
    let key = idempotency_key(&request)?;
    let address = request_address(&request, &state)?;
    let path = path_of(&request);
    let (body, raw) = read_json::<LoginRequest>(request).await?;
    let pool = pool_of(&state)?;
    let scope = request_scope(&Method::POST, &path, &address);
    let guard = guard_idem(pool, &key, &raw, &scope).await?;
    let limiter = Limiter::new(pool, std::sync::Arc::clone(&state.clock));
    let attempt = limiter
        .begin_login(&address, &body.account)
        .await
        .map_err(internal_sqlx)?;
    let Some(id) = attempt else {
        slow_fail(&state).await;
        return Err(denied(
            StatusCode::TOO_MANY_REQUESTS,
            MessageCode::AccountLoginRate,
        ));
    };
    if let Guard::Replay(..) = guard {
        return run_login(&state, pool, &limiter, id, &body, &key, &raw, &scope).await;
    }
    run_login(&state, pool, &limiter, id, &body, &key, &raw, &scope).await
}

async fn run_login(
    state: &AppState,
    pool: &Pool,
    limiter: &Limiter,
    id: i64,
    body: &LoginRequest,
    key: &str,
    raw: &[u8],
    scope: &str,
) -> Result<Response, CallerDenied> {
    eprintln!("ENTER {}", key);
    let account = normalize_account_name(&body.account)
        .map_err(|_| denied(StatusCode::UNAUTHORIZED, MessageCode::AccountLoginCredentials))?;
    let row = accounts::find_account(pool, &account)
        .await
        .map_err(internal_sqlx)?;
    let checked = verify_password(
        state,
        row.as_ref(),
        &Secret::from_untrusted(body.password.clone()),
    )
    .await;
    let ok = checked && row.is_some();
    if !ok {
        eprintln!("SLOW {}", key);
        slow_fail(state).await;
        eprintln!("SLEPT {}", key);
        let value = serde_json::to_value(problems::auth_problem(
            StatusCode::UNAUTHORIZED.as_u16(),
            MessageCode::AccountLoginCredentials,
        ))
        .map_err(internal_text)?;
        let answer = record_outcome(pool, state, key, raw, scope, StatusCode::UNAUTHORIZED, &value, false).await?;
        drop(answer);
        return Err(denied(StatusCode::UNAUTHORIZED, MessageCode::AccountLoginCredentials));
    }
    if row.as_ref().is_some_and(|row| row.disabled) {
        let value = serde_json::to_value(problems::auth_problem(
            StatusCode::FORBIDDEN.as_u16(),
            MessageCode::AccountLoginDisabled,
        ))
        .map_err(internal_text)?;
        limiter
            .cancel_login(id)
            .await
            .map_err(internal_sqlx)?;
        record_outcome(pool, state, key, raw, scope, StatusCode::FORBIDDEN, &value, false).await?;
        return Err(denied(StatusCode::FORBIDDEN, MessageCode::AccountLoginDisabled));
    }
    limiter
        .cancel_login(id)
        .await
        .map_err(internal_sqlx)?;
    let session = open_session(state, pool, body, row.as_ref()).await?;
    let value = value_of(&session)?;
    record_outcome(pool, state, key, raw, scope, StatusCode::OK, &value, true).await
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
    let stored = find_recorded(pool, key, scope).await.map_err(internal_sqlx)?;
    match stored {
        None => Ok(Guard::Fresh),
        Some(record) if record.request_hash == safe_digest(raw) => Ok(Guard::Replay(
            status_from_i32(record.status_code)?,
            serde_json::from_str(&record.body).unwrap_or(serde_json::Value::Null),
            record.body == DID_IT_BODY,
        )),
        Some(_) => Err(denied(
            StatusCode::CONFLICT,
            MessageCode::AccountIdempotencyConflict,
        )),
    }
}

async fn verify_password(
    state: &AppState,
    row: Option<&accounts::AccountRow>,
    password: &Secret,
) -> bool {
    let Ok(hasher) = state.pass_hasher().await else {
        return false;
    };
    match row {
        Some(row) => hasher
            .verify(password, &row.password_hash)
            .await
            .unwrap_or(false),
        None => hasher.verify_dummy(password).await.unwrap_or(false),
    }
}

async fn open_session(
    state: &AppState,
    pool: &Pool,
    body: &LoginRequest,
    row: Option<&accounts::AccountRow>,
) -> Result<LoginSession, CallerDenied> {
    let account = row.ok_or(denied(
        StatusCode::UNAUTHORIZED,
        MessageCode::AccountLoginCredentials,
    ))?;
    let now_ms = state.clock.now_ms().millis();
    let at_ms = i64::try_from(now_ms).map_err(internal_text)?;
    trim_sessions(pool, &account.account, now_ms).await?;
    let key = Secret::from_untrusted(crate::session_key::random_key()?);
    sessions::insert_session(
        pool,
        &uuid::Uuid::now_v7().to_string(),
        &account.account,
        &key_digest(&key),
        &body.label,
        at_ms,
    )
    .await
    .map_err(internal_sqlx)?;
    Ok(LoginSession {
        session_key: key.expose().to_owned(),
        account: account.account.clone(),
    })
}

async fn trim_sessions(pool: &Pool, account: &str, now_ms: u64) -> Result<(), CallerDenied> {
    let rows = sessions::list_sessions(pool, account)
        .await
        .map_err(internal_sqlx)?;
    let mut doomed: Vec<String> = Vec::new();
    let mut alive: Vec<(u64, String)> = Vec::new();
    for row in rows {
        let last = u64::try_from(row.last_active_ms).map_err(internal_text)?;
        if session_expired(last, now_ms) {
            doomed.push(row.id);
        } else {
            alive.push((last, row.id));
        }
    }
    let times: Vec<u64> = alive.iter().map(|(at, _)| *at).collect();
    let doomed_indices =
        evict_indices(alive.len().saturating_add(1), &times, now_ms).map_err(internal_text)?;
    for index in doomed_indices {
        if let Some((_, id)) = alive.get(index) {
            doomed.push(id.clone());
        }
    }
    let refs: Vec<&str> = doomed.iter().map(String::as_str).collect();
    sessions::delete_sessions(pool, &refs)
        .await
        .map_err(internal_sqlx)
}

async fn slow_fail(state: &AppState) {
    state
        .clock
        .sleep(std::time::Duration::from_millis(LOGIN_DELAY_MS))
        .await;
}
