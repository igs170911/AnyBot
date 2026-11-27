use axum::http::StatusCode;

use anybot_foundation::Secret;
use anybot_messages::MessageCode;
use anybot_store_server::computers::{self, ComputerRow};
use anybot_store_server::{Pool, accounts};

use crate::app::AppState;
use crate::caller::{Caller, CallerDenied, CredentialKind, denied_code, internal};
use crate::caller_session::pool_of_state;
use crate::computer_view::{signed, unsigned};
use crate::pass_hash::key_digest;

pub(crate) async fn resolve_device(
    secret: &Secret,
    address: &str,
    state: &AppState,
) -> Result<Caller, CallerDenied> {
    let pool = pool_of_state(state)?;
    let found = computers::find_by_key_hash(&pool, &key_digest(secret))
        .await
        .map_err(internal)?;
    let Some(row) = found else {
        return Err(denied_code(
            StatusCode::UNAUTHORIZED,
            MessageCode::AccountLoginCredentials,
        ));
    };
    if row.removed_at_ms.is_some() {
        return Err(denied_code(
            StatusCode::UNAUTHORIZED,
            MessageCode::ComputerRemoved,
        ));
    }
    let account = accounts::find_account(&pool, &row.account)
        .await
        .map_err(internal)?
        .ok_or(denied_code(
            StatusCode::UNAUTHORIZED,
            MessageCode::AccountLoginCredentials,
        ))?;
    if account.disabled {
        return Err(denied_code(
            StatusCode::UNAUTHORIZED,
            MessageCode::AccountLoginDisabled,
        ));
    }
    note_signal(state, &pool, &row).await?;
    Ok(Caller {
        account: row.account.clone(),
        admin: account.admin && !row.paired,
        session_id: String::new(),
        device_id: row.device_id,
        credential: CredentialKind::Computer,
        address: address.to_owned(),
        paired: row.paired,
    })
}

async fn note_signal(state: &AppState, pool: &Pool, row: &ComputerRow) -> Result<(), CallerDenied> {
    let now = state.clock.now_ms().millis();
    let at = signed(now)?;
    let write_due = state
        .presence
        .note_request(&row.device_id, unsigned(row.last_seen_ms)?, now);
    if !row.online {
        let flipped = computers::mark_online(pool, &row.device_id, at)
            .await
            .map_err(internal)?;
        state.presence.note_written(&row.device_id, now);
        if flipped {
            crate::computers_log::announce(state, pool, &row.account, "online", &row.device_id)
                .await?;
        }
    } else if write_due {
        computers::touch_seen(pool, &row.device_id, at)
            .await
            .map_err(internal)?;
        state.presence.note_written(&row.device_id, now);
    }
    if row.credential_used_ms.is_none() {
        computers::mark_credential_used(pool, &row.device_id, at)
            .await
            .map_err(internal)?;
    }
    Ok(())
}
