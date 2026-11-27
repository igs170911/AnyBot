use anybot_domain::computer_name::clean_name;
use anybot_domain::pairing_rules::{code_has_valid_shape, code_is_alive};
use anybot_foundation::Secret;
use anybot_protocol::{LoginSession, PairRedeemRequest};
use anybot_store_server::pairing_codes::{self, CodeRow};
use anybot_store_server::{HashPurpose, Pool, computers};

use crate::app::AppState;
use crate::computer_view::{signed, unsigned};
use crate::computers_join::{JoinSpec, join_account, name_denied};
use crate::pass_hash::key_digest;
use crate::req_write::{CallerDenied, internal_detail, internal_sqlx, internal_text, pool_of};

pub(crate) struct Redeem<'a> {
    pub key: &'a str,
    pub address: &'a str,
    pub body: &'a PairRedeemRequest,
}

pub(crate) async fn redeem_code(
    state: &AppState,
    redeem: &Redeem<'_>,
) -> Result<Option<LoginSession>, CallerDenied> {
    clean_name(&redeem.body.name).map_err(name_denied)?;
    if !code_has_valid_shape(&redeem.body.code) {
        return Ok(None);
    }
    let pool = pool_of(state)?;
    let keys = state
        .secrets
        .as_ref()
        .ok_or_else(|| internal_detail("secret keys are not loaded"))?;
    let hash = keys
        .keyed_hash(
            HashPurpose::PairingCode,
            "",
            &Secret::from_untrusted(redeem.body.code.clone()),
        )
        .map_err(internal_text)?;
    let found = pairing_codes::find_by_hash(pool, &hash.algorithm, &hash.digest)
        .await
        .map_err(internal_sqlx)?;
    let Some(row) = found else {
        return Ok(None);
    };
    let now = state.clock.now_ms().millis();
    if row.used_at_ms.is_none() {
        return first_redeem(state, pool, &row, redeem, now).await;
    }
    second_redeem(pool, &row, redeem, now).await
}

async fn first_redeem(
    state: &AppState,
    pool: &Pool,
    row: &CodeRow,
    redeem: &Redeem<'_>,
    now: u64,
) -> Result<Option<LoginSession>, CallerDenied> {
    if !code_is_alive(unsigned(row.expires_at_ms)?, now) {
        return Ok(None);
    }
    let claimed = pairing_codes::claim(pool, &row.id, redeem.address, signed(now)?)
        .await
        .map_err(internal_sqlx)?;
    if !claimed {
        return Ok(None);
    }
    let joined = join_account(
        state,
        pool,
        &JoinSpec {
            account: &row.account,
            label: &redeem.body.name,
            paired: true,
        },
    )
    .await?;
    pairing_codes::set_redeemed_device(pool, &row.id, &joined.device_id)
        .await
        .map_err(internal_sqlx)?;
    pairing_codes::record_redeem(
        pool,
        redeem.key,
        redeem.address,
        &row.id,
        &joined.device_id,
        signed(now)?,
    )
    .await
    .map_err(internal_sqlx)?;
    Ok(Some(session_of(
        &row.account,
        joined.device_id,
        &joined.key,
    )))
}

async fn second_redeem(
    pool: &Pool,
    row: &CodeRow,
    redeem: &Redeem<'_>,
    now: u64,
) -> Result<Option<LoginSession>, CallerDenied> {
    if !code_is_alive(unsigned(row.expires_at_ms)?, now) {
        return Ok(None);
    }
    let earlier = pairing_codes::redeem_of(pool, redeem.key, redeem.address)
        .await
        .map_err(internal_sqlx)?;
    let Some((code_id, device_id)) = earlier else {
        return Ok(None);
    };
    if code_id != row.id {
        return Ok(None);
    }
    let key = Secret::from_untrusted(crate::session_key::random_key()?);
    let replaced = replace_unused_key(pool, &row.account, &device_id, &key).await?;
    Ok(replaced.then(|| session_of(&row.account, device_id, &key)))
}

async fn replace_unused_key(
    pool: &Pool,
    account: &str,
    device_id: &str,
    key: &Secret,
) -> Result<bool, CallerDenied> {
    let mut tx = pool.begin().await.map_err(internal_sqlx)?;
    computers::lock_account_in(&mut tx, account)
        .await
        .map_err(internal_sqlx)?;
    let replaced = computers::replace_unused_key_in(&mut tx, device_id, &key_digest(key))
        .await
        .map_err(internal_sqlx)?;
    tx.commit().await.map_err(internal_sqlx)?;
    Ok(replaced)
}

fn session_of(account: &str, device_id: String, key: &Secret) -> LoginSession {
    LoginSession {
        session_key: None,
        bearer_key: Some(key.expose().to_owned()),
        account: account.to_owned(),
        device_id,
    }
}
