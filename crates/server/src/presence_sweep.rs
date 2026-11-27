use std::sync::Arc;
use std::time::Duration;

use tokio::task::JoinHandle;

use anybot_domain::computer_presence::{OfflineReason, Presence, SWEEP_SPAN_MS};
use anybot_store_server::computers;

use crate::app::AppState;
use crate::caller_session::pool_of_state;
use crate::computer_view::{presence_now, signed};
use crate::computers_log::announce;
use crate::req_write::{CallerDenied, internal_sqlx};

#[must_use]
pub fn spawn_sweeper(state: AppState) -> JoinHandle<()> {
    tokio::spawn(run_sweeper(state))
}

async fn run_sweeper(state: AppState) {
    let shutdown = Arc::clone(&state.shutdown);
    let stop = shutdown.notified();
    tokio::pin!(stop);
    loop {
        tokio::select! {
            biased;
            () = &mut stop => break,
            () = state.clock.sleep(Duration::from_millis(SWEEP_SPAN_MS)) => {
                report(&sweep_once(&state).await);
            }
        }
    }
    report(&finish(&state).await);
}

fn report(outcome: &Result<(), CallerDenied>) {
    if outcome.is_err() {
        tracing::warn!("computer presence sweep failed");
    }
}

async fn finish(state: &AppState) -> Result<(), CallerDenied> {
    flush_seen(state).await?;
    sweep_once(state).await
}

pub(crate) async fn flush_seen(state: &AppState) -> Result<(), CallerDenied> {
    let pool = pool_of_state(state)?;
    for (device_id, seen_ms) in state.presence.unwritten() {
        computers::touch_seen(&pool, &device_id, signed(seen_ms)?)
            .await
            .map_err(internal_sqlx)?;
        state.presence.note_written(&device_id, seen_ms);
    }
    Ok(())
}

pub(crate) async fn sweep_once(state: &AppState) -> Result<(), CallerDenied> {
    let pool = pool_of_state(state)?;
    let now_ms = state.clock.now_ms().millis();
    let rows = computers::list_all_live(&pool)
        .await
        .map_err(internal_sqlx)?;
    for row in rows.iter().filter(|row| row.online) {
        let Presence::Offline { at_ms, reason } = presence_now(state, row, now_ms)? else {
            continue;
        };
        let flipped = computers::mark_offline(
            &pool,
            &row.device_id,
            signed(at_ms)?,
            reason.map(OfflineReason::code),
        )
        .await
        .map_err(internal_sqlx)?;
        if flipped {
            announce(state, &pool, &row.account, "offline", &row.device_id).await?;
        }
    }
    Ok(())
}
