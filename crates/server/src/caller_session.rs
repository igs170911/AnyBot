use axum::http::StatusCode;

use anybot_domain::session_term::session_expired;
use anybot_foundation::Secret;
use anybot_messages::MessageCode;

use anybot_store_server::{Pool, accounts, sessions};

use crate::app::AppState;
use crate::caller::{
    Caller, CredentialKind, denied_code, internal, internal_detail, internal_static,
};
use crate::pass_hash::key_digest;

pub(crate) fn pool_of_state(state: &AppState) -> Result<Pool, crate::caller::CallerDenied> {
    state
        .pool
        .as_ref()
        .clone()
        .ok_or_else(|| internal_static("no database pool"))
}

pub(crate) async fn resolve_session(
    credential: (CredentialKind, Secret),
    address: &str,
    state: &AppState,
) -> Result<Caller, crate::caller::CallerDenied> {
    let pool = pool_of_state(state)?;
    let row = sessions::find_session_by_key_hash(&pool, &key_digest(&credential.1))
        .await
        .map_err(internal)?;
    let row = row.ok_or(denied_code(
        StatusCode::UNAUTHORIZED,
        MessageCode::AccountLoginCredentials,
    ))?;
    let now_ms = state.clock.now_ms().millis();
    let last_active =
        u64::try_from(row.last_active_ms).map_err(|e| internal_detail(&e.to_string()))?;
    if session_expired(last_active, now_ms) {
        return Err(denied_code(
            StatusCode::UNAUTHORIZED,
            MessageCode::AccountLoginCredentials,
        ));
    }
    let account = accounts::find_account(&pool, &row.account)
        .await
        .map_err(internal)?;
    let account = account.ok_or(denied_code(
        StatusCode::UNAUTHORIZED,
        MessageCode::AccountLoginCredentials,
    ))?;
    if account.disabled {
        return Err(denied_code(
            StatusCode::UNAUTHORIZED,
            MessageCode::AccountLoginDisabled,
        ));
    }
    if row.last_password_at_ms != account.last_password_at_ms {
        return Err(denied_code(
            StatusCode::UNAUTHORIZED,
            MessageCode::AccountLoginCredentials,
        ));
    }
    let Some(device_id) = row.device_id.clone() else {
        return Err(denied_code(
            StatusCode::UNAUTHORIZED,
            MessageCode::AccountLoginCredentials,
        ));
    };
    crate::caller::touch_if_due(&pool, &row, now_ms).await?;
    Ok(Caller {
        account: row.account,
        admin: account.admin,
        session_id: row.id,
        device_id,
        credential: credential.0,
        address: address.to_owned(),
    })
}
