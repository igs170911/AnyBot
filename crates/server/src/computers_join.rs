use axum::http::StatusCode;

use anybot_domain::computer_name::{NameError, clean_name, name_with_smallest_tail};
use anybot_domain::computer_presence::{REASON_REMOVED, evict_oldest_seen};
use anybot_foundation::Secret;
use anybot_messages::MessageCode;
use anybot_store_server::computers::{self, ComputerRow, NewComputer};
use anybot_store_server::{Pool, computers::lock_account_in};

use crate::app::AppState;
use crate::computer_view::{is_online, signed, unsigned};
use crate::computers_remove::finish_removal;
use crate::pass_hash::key_digest;
use crate::req_write::{CallerDenied, denied, internal_sqlx};

pub(crate) struct JoinSpec<'a> {
    pub account: &'a str,
    pub label: &'a str,
    pub paired: bool,
}

pub(crate) struct Joined {
    pub device_id: String,
    pub key: Secret,
}

struct JoinPlan {
    name: String,
    removed: Vec<String>,
}

pub(crate) fn name_denied(error: NameError) -> CallerDenied {
    let code = match error {
        NameError::TooLong => MessageCode::ComputerNameTooLong,
        NameError::Empty => MessageCode::ComputerNameEmpty,
    };
    denied(StatusCode::BAD_REQUEST, code)
}

pub(crate) async fn join_account(
    state: &AppState,
    pool: &Pool,
    spec: &JoinSpec<'_>,
) -> Result<Joined, CallerDenied> {
    let wanted = clean_name(spec.label).map_err(name_denied)?;
    let key = Secret::from_untrusted(crate::session_key::random_key()?);
    let device_id = uuid::Uuid::now_v7().to_string();
    let now = state.clock.now_ms().millis();
    let mut tx = pool.begin().await.map_err(internal_sqlx)?;
    lock_account_in(&mut tx, spec.account)
        .await
        .map_err(internal_sqlx)?;
    let live = computers::list_live_in(&mut tx, spec.account)
        .await
        .map_err(internal_sqlx)?;
    let plan = plan_join(state, &live, &wanted, spec.paired, now)?;
    for doomed in &plan.removed {
        computers::mark_removed_in(&mut tx, doomed, signed(now)?, REASON_REMOVED)
            .await
            .map_err(internal_sqlx)?;
    }
    computers::insert_in(
        &mut tx,
        &NewComputer {
            device_id: &device_id,
            account: spec.account,
            name: &plan.name,
            key_hash: &key_digest(&key),
            paired: spec.paired,
            now_ms: signed(now)?,
        },
    )
    .await
    .map_err(internal_sqlx)?;
    tx.commit().await.map_err(internal_sqlx)?;
    state.presence.note_request(&device_id, now, now);
    for doomed in &plan.removed {
        finish_removal(state, pool, spec.account, doomed).await?;
    }
    crate::computers_log::announce(state, pool, spec.account, "added", &device_id).await?;
    Ok(Joined { device_id, key })
}

fn plan_join(
    state: &AppState,
    live: &[ComputerRow],
    wanted: &str,
    paired: bool,
    now: u64,
) -> Result<JoinPlan, CallerDenied> {
    let names: Vec<String> = live.iter().map(|row| row.name.clone()).collect();
    let holder = live.iter().find(|row| row.name == wanted);
    let mut removed: Vec<String> = Vec::new();
    let mut name = wanted.to_owned();
    if let Some(holder) = holder {
        if paired || is_online(state, holder, now)? {
            name = name_with_smallest_tail(wanted, &names);
        } else {
            removed.push(holder.device_id.clone());
        }
    }
    if let Some(oldest) = oldest_to_evict(state, live, &removed)? {
        removed.push(oldest);
    }
    Ok(JoinPlan { name, removed })
}

fn oldest_to_evict(
    state: &AppState,
    live: &[ComputerRow],
    removed: &[String],
) -> Result<Option<String>, CallerDenied> {
    let rest: Vec<&ComputerRow> = live
        .iter()
        .filter(|row| !removed.contains(&row.device_id))
        .collect();
    let mut seen: Vec<u64> = Vec::new();
    for row in &rest {
        let stored = unsigned(row.last_seen_ms)?;
        let newest = state
            .presence
            .last_request(&row.device_id)
            .map_or(stored, |live_seen| live_seen.max(stored));
        seen.push(newest);
    }
    let index = evict_oldest_seen(&seen);
    Ok(index
        .and_then(|at| rest.get(at))
        .map(|row| row.device_id.clone()))
}
