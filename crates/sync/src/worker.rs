use std::sync::Arc;
use std::time::Duration;

use crate::core::Core;
use crate::error::SyncError;
use crate::flush::{Flush, flush_once};

const IDLE_TICK: Duration = Duration::from_secs(3600);

pub(crate) async fn run(core: Arc<Core>) {
    loop {
        if core.halted() {
            core.retry_now.notified().await;
            continue;
        }
        match cycle(&core).await {
            Ok(Flush::Sent) => {
                clear_error(&core);
            }
            Ok(Flush::Idle) => {
                clear_error(&core);
                idle(&core).await;
            }
            Ok(Flush::Halted) => {}
            Ok(Flush::Retry) => backoff(&core).await,
            Err(error) => {
                report(&core, &error);
                backoff(&core).await;
            }
        }
    }
}

async fn cycle(core: &Core) -> Result<Flush, SyncError> {
    flush_once(core).await
}

fn clear_error(core: &Core) {
    core.status.update(|status| status.local_error = None);
}

fn report(core: &Core, error: &SyncError) {
    let text = error.to_string();
    core.status.update(|status| status.local_error = Some(text));
}

async fn idle(core: &Core) {
    let tick = core.clock.sleep(IDLE_TICK);
    tokio::select! {
        () = core.wake.notified() => {}
        () = tick => {}
    }
}

pub(crate) async fn backoff(core: &Core) {
    let delay = core.backoff_delay();
    let wake = core.clock.sleep(delay);
    tokio::select! {
        () = wake => {}
        () = core.retry_now.notified() => {}
    }
}
