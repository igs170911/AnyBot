use std::sync::Arc;
use std::time::Duration;

use anybot_store_client::ACCOUNT_STREAM;

use crate::core::Core;
use crate::error::SyncError;
use crate::flush::{BOT_CREATE_KIND, Flush, flush_once};
use crate::pull::{Pulled, pull_stream};
use crate::resync::resync_stream;
use crate::wanted::Job;

const IDLE_TICK: Duration = Duration::from_secs(3600);

pub(crate) async fn run(core: Arc<Core>) {
    loop {
        if core.halted() {
            core.retry_now.notified().await;
            continue;
        }
        match cycle(&core).await {
            Ok(Step::Again) => clear_error(&core),
            Ok(Step::Rest) => {
                clear_error(&core);
                idle(&core).await;
            }
            Ok(Step::Halted) => {}
            Ok(Step::Backoff) => backoff(&core).await,
            Err(error) => {
                report(&core, &error);
                backoff(&core).await;
            }
        }
    }
}

enum Step {
    Again,
    Rest,
    Halted,
    Backoff,
}

async fn cycle(core: &Core) -> Result<Step, SyncError> {
    match flush_once(core).await? {
        Flush::Sent => Ok(Step::Again),
        Flush::Halted => Ok(Step::Halted),
        Flush::Retry => Ok(Step::Backoff),
        Flush::Idle => fetch_once(core).await,
    }
}

async fn fetch_once(core: &Core) -> Result<Step, SyncError> {
    let Some(job) = core.wanted.next() else {
        return Ok(Step::Rest);
    };
    let (stream, outcome, resync) = match job {
        Job::Pull(stream) => {
            let outcome = pull_stream(core, &stream).await?;
            (stream, outcome, false)
        }
        Job::Resync(stream) => {
            let outcome = resync_stream(core, &stream).await?;
            (stream, outcome, true)
        }
    };
    match outcome {
        Pulled::Caught => {
            discover_bots(core, &stream).await?;
            Ok(Step::Again)
        }
        Pulled::Mismatch if !resync => {
            core.wanted.resync(&stream);
            Ok(Step::Again)
        }
        Pulled::Mismatch | Pulled::Refused => Ok(Step::Again),
        Pulled::Retry => {
            requeue(core, &stream, resync);
            Ok(Step::Backoff)
        }
        Pulled::Halted => {
            requeue(core, &stream, resync);
            Ok(Step::Halted)
        }
    }
}

fn requeue(core: &Core, stream: &str, resync: bool) {
    if resync {
        core.wanted.resync(stream);
    } else {
        core.wanted.pull(stream);
    }
}

async fn discover_bots(core: &Core, stream: &str) -> Result<(), SyncError> {
    if stream != ACCOUNT_STREAM {
        return Ok(());
    }
    let known = core.store.replica_streams().await?;
    for record in core.store.records(ACCOUNT_STREAM).await? {
        if record.kind != BOT_CREATE_KIND {
            continue;
        }
        let Some(bot_id) = record
            .content
            .get("bot_id")
            .and_then(serde_json::Value::as_str)
        else {
            continue;
        };
        let key = anybot_store_client::bot_stream(bot_id);
        if !known.contains(&key) {
            core.wanted.pull(&key);
        }
    }
    Ok(())
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
