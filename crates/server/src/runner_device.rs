use axum::http::StatusCode;

use anybot_foundation::Secret;
use anybot_messages::MessageCode;
use anybot_store_server::runner_device::{self, NewRunnerDevice};

use crate::app::AppState;
use crate::caller::{Caller, CallerDenied, CredentialKind, denied_code, internal};
use crate::caller_session::pool_of_state;
use crate::computer_view::signed;
use crate::pass_hash::key_digest;
use crate::two_step_check::secrets_of;

pub async fn ensure_runner_device(state: &AppState) -> Result<(), String> {
    let pool = pool_of_state(state).map_err(|_| "no database pool".to_owned())?;
    let keys = secrets_of(state).map_err(|_| "no secret keys".to_owned())?;
    let existing = runner_device::find(&pool)
        .await
        .map_err(|e| e.to_string())?;
    if existing.is_some() {
        return Ok(());
    }
    let key = Secret::from_untrusted(crate::session_key::random_key().map_err(|_| "no random")?);
    let device_id = uuid::Uuid::now_v7().to_string();
    let now = signed(state.clock.now_ms().millis()).map_err(|_| "clock out of range")?;
    runner_device::insert_if_absent(
        &pool,
        &keys,
        &NewRunnerDevice {
            device_id: &device_id,
            key: &key,
            key_hash: &key_digest(&key),
            now_ms: now,
        },
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub(crate) async fn resolve_runner(
    secret: &Secret,
    address: &str,
    state: &AppState,
) -> Result<Caller, CallerDenied> {
    let pool = pool_of_state(state)?;
    let found = runner_device::find_by_key_hash(&pool, &key_digest(secret))
        .await
        .map_err(internal)?;
    let Some(row) = found else {
        return Err(denied_code(
            StatusCode::UNAUTHORIZED,
            MessageCode::AccountLoginCredentials,
        ));
    };
    Ok(Caller {
        account: String::new(),
        admin: false,
        session_id: String::new(),
        device_id: row.device_id,
        credential: CredentialKind::Runner,
        address: address.to_owned(),
        paired: false,
    })
}
