use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Notify;

use anybot_foundation::{Clock, TimestampMs};
use anybot_store_server::{Pool, auth_events, idempotency};

pub const CLEANUP_SPAN: Duration = Duration::from_secs(3600);
pub const AUTH_RETENTION_MS: u64 = 24 * 3600 * 1000;
pub const IDEM_RETENTION_MS: u64 = 7 * 24 * 3600 * 1000;

pub fn cleanup_loop(
    pool: Pool,
    clock: Arc<dyn Clock>,
    shutdown: Arc<Notify>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(run_cleanup(pool, clock, shutdown))
}

async fn run_cleanup(pool: Pool, clock: Arc<dyn Clock>, shutdown: Arc<Notify>) {
    loop {
        tokio::select! {
            biased;
            () = shutdown.notified() => break,
            () = clock.sleep(CLEANUP_SPAN) => {
                let _round = prune_every(&pool, &clock).await;
            }
        }
    }
}

async fn prune_every(pool: &Pool, clock: &Arc<dyn Clock>) -> Result<(), sqlx::Error> {
    let auth_floor = retention_floor(clock.now_ms(), AUTH_RETENTION_MS)?;
    let idem_floor = retention_floor(clock.now_ms(), IDEM_RETENTION_MS)?;
    let _auth = auth_events::prune_auth_events_before(pool, auth_floor).await?;
    let _idem = idempotency::prune_idempotency_before(pool, idem_floor).await?;
    Ok(())
}

fn retention_floor(now: TimestampMs, span_ms: u64) -> Result<i64, sqlx::Error> {
    i64::try_from(now.millis().saturating_sub(span_ms))
        .map_err(|e| sqlx::Error::Protocol(e.to_string()))
}
