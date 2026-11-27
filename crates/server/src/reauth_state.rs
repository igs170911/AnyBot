use axum::http::StatusCode;

use anybot_domain::pairing_rules::password_is_fresh;
use anybot_messages::MessageCode;
use anybot_store_server::{Pool, computers, sessions_reauth};

use crate::app::AppState;
use crate::caller::{Caller, CredentialKind};
use crate::computer_view::{signed, unsigned};
use crate::req_write::{CallerDenied, denied, internal_sqlx};

pub(crate) async fn require_fresh_password(
    state: &AppState,
    pool: &Pool,
    caller: &Caller,
) -> Result<(), CallerDenied> {
    let entered = entered_ms(pool, caller).await?;
    let now = state.clock.now_ms().millis();
    if password_is_fresh(unsigned(entered)?, now) {
        return Ok(());
    }
    Err(denied(
        StatusCode::FORBIDDEN,
        MessageCode::AccountReauthRequired,
    ))
}

async fn entered_ms(pool: &Pool, caller: &Caller) -> Result<i64, CallerDenied> {
    let found = if caller.credential == CredentialKind::Computer {
        computers::find_by_device(pool, &caller.device_id)
            .await
            .map_err(internal_sqlx)?
            .map(|row| row.password_entered_ms)
    } else {
        sessions_reauth::password_entered_ms(pool, &caller.session_id)
            .await
            .map_err(internal_sqlx)?
    };
    found.ok_or(denied(
        StatusCode::UNAUTHORIZED,
        MessageCode::AccountLoginCredentials,
    ))
}

pub(crate) async fn record_password_entry(
    state: &AppState,
    pool: &Pool,
    caller: &Caller,
) -> Result<(), CallerDenied> {
    let now = signed(state.clock.now_ms().millis())?;
    if caller.credential == CredentialKind::Computer {
        computers::enter_password(pool, &caller.device_id, now)
            .await
            .map_err(internal_sqlx)
    } else {
        sessions_reauth::enter_password(pool, &caller.session_id, now)
            .await
            .map_err(internal_sqlx)
    }
}
