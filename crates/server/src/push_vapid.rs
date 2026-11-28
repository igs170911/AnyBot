use anybot_channels::web_push::VapidKey;
use anybot_foundation::Secret;
use anybot_store_server::{Pool, SecretKeys, push_key};

use crate::app::AppState;
use crate::caller_session::pool_of_state;
use crate::computer_view::signed;
use crate::two_step_check::secrets_of;

pub struct ServerVapid {
    pub key: VapidKey,
    pub public_key: String,
}

pub async fn ensure_vapid(
    pool: &Pool,
    keys: &SecretKeys,
    now_ms: i64,
) -> Result<ServerVapid, String> {
    if let Some(found) = load_vapid(pool, keys).await? {
        return Ok(found);
    }
    let fresh = VapidKey::generate().map_err(|e| e.to_string())?;
    let public = fresh.public_b64().map_err(|e| e.to_string())?;
    let private = Secret::from_untrusted(fresh.scalar().expose().to_owned());
    push_key::insert_if_absent(pool, keys, &public, &private, now_ms)
        .await
        .map_err(|e| e.to_string())?;
    load_vapid(pool, keys)
        .await?
        .ok_or_else(|| "push server key vanished after insert".to_owned())
}

async fn load_vapid(pool: &Pool, keys: &SecretKeys) -> Result<Option<ServerVapid>, String> {
    let Some(stored) = push_key::load(pool, keys)
        .await
        .map_err(|e| e.to_string())?
    else {
        return Ok(None);
    };
    let key = VapidKey::from_scalar(stored.private).map_err(|e| e.to_string())?;
    Ok(Some(ServerVapid {
        key,
        public_key: stored.public_key,
    }))
}

pub async fn ensure_push_key(state: &AppState) -> Result<(), String> {
    let pool = pool_of_state(state).map_err(|_| "no database pool".to_owned())?;
    let keys = secrets_of(state).map_err(|_| "no secret keys".to_owned())?;
    let now = signed(state.clock.now_ms().millis()).map_err(|_| "clock out of range".to_owned())?;
    ensure_vapid(&pool, &keys, now).await.map(|_| ())
}
