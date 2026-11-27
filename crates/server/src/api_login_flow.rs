use anybot_domain::session_term::{evict_indices, session_expired};
use anybot_foundation::Secret;
use anybot_protocol::{LoginRequest, LoginSession};
use anybot_store_server::{Pool, accounts, sessions};

use crate::app::AppState;
use crate::caller::CallerDenied;
use crate::pass_hash::key_digest;
use crate::req_write::{internal_detail, internal_sqlx, internal_text};

const LOGIN_DELAY_MS: u64 = 500;

pub(crate) async fn verify_password(
    state: &AppState,
    row: Option<&accounts::AccountRow>,
    password: &Secret,
) -> Result<bool, CallerDenied> {
    let hasher = state.pass_hasher().await.map_err(|e| internal_detail(&e))?;
    match row {
        Some(row) => hasher.verify(password, &row.password_hash).await,
        None => hasher.verify_dummy(password).await,
    }
    .map_err(|e| internal_detail(&e))
}

pub(crate) fn credential_is_cookie(body: &LoginRequest) -> Result<bool, CallerDenied> {
    match body.credential.as_str() {
        "cookie" => Ok(true),
        "bearer" | "computer" => Ok(false),
        _ => Err(crate::caller::denied_code(
            axum::http::StatusCode::UNAUTHORIZED,
            anybot_messages::MessageCode::AccountLoginCredentials,
        )),
    }
}

pub(crate) fn credential_is_computer(body: &LoginRequest) -> bool {
    body.credential == crate::two_step_challenge::CREDENTIAL_COMPUTER
}

pub(crate) async fn open_session(
    state: &AppState,
    pool: &Pool,
    label: &str,
    account: &str,
    cookie_mode: bool,
    used_recovery: bool,
) -> Result<LoginSession, CallerDenied> {
    let now_ms = state.clock.now_ms().millis();
    let at_ms = i64::try_from(now_ms).map_err(internal_text)?;
    trim_sessions(pool, account, now_ms).await?;
    let key = Secret::from_untrusted(crate::session_key::random_key()?);
    let device_id = uuid::Uuid::now_v7().to_string();
    sessions::insert_session(
        pool,
        &uuid::Uuid::now_v7().to_string(),
        account,
        &key_digest(&key),
        label,
        &device_id,
        at_ms,
    )
    .await
    .map_err(internal_sqlx)?;
    let bearer_key = (!cookie_mode).then(|| key.expose().to_owned());
    Ok(LoginSession {
        session_key: Some(key.expose().to_owned()),
        bearer_key,
        account: account.to_owned(),
        device_id,
        used_recovery: used_recovery.then_some(true),
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

pub(crate) async fn slow_fail(state: &AppState) {
    state
        .clock
        .sleep(std::time::Duration::from_millis(LOGIN_DELAY_MS))
        .await;
}
