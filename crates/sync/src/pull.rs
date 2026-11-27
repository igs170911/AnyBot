use anybot_protocol::{AfterPage, ProblemJson, StoredRecord};
use anybot_store_client::{ACCOUNT_STREAM, ApplyOutcome};
use url::form_urlencoded;

use crate::core::Core;
use crate::error::SyncError;
use crate::reply_gate::{Gate, gate};
use crate::wire::{WireLimits, WireMethod, WireReply, WireRequest};

pub(crate) const AHEAD_CODE: &str = "sync-after-ahead";
const STALLS_BEFORE_RETRY: u32 = 5;
const ACCOUNT_KEY: &str = "account/me";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pulled {
    Caught,
    Mismatch,
    Retry,
    Halted,
    Refused,
}

pub(crate) fn stream_key(stream: &str) -> String {
    if stream == ACCOUNT_STREAM {
        ACCOUNT_KEY.to_owned()
    } else {
        stream.to_owned()
    }
}

pub(crate) fn after_path(stream: &str, after: i64, record_id: &str) -> String {
    let mut query = form_urlencoded::Serializer::new(String::new());
    query.append_pair("stream", &stream_key(stream));
    query.append_pair("after", &after.to_string());
    if after > 0 {
        query.append_pair("device_at", record_id);
    }
    format!("/v1/streams/after?{}", query.finish())
}

pub(crate) enum Fetch {
    Page(AfterPage),
    Mismatch,
    Failed(Pulled),
}

pub(crate) async fn fetch_page(
    core: &Core,
    stream: &str,
    after: i64,
    record_id: &str,
) -> Result<Fetch, SyncError> {
    let request = WireRequest {
        method: WireMethod::Get,
        path: after_path(stream, after, record_id),
        body: None,
        idempotency_key: None,
        limits: WireLimits::BACKGROUND,
    };
    match core.wire.call(request).await {
        Ok(reply) => {
            core.note_reply(&reply)?;
            read_reply(core, &reply)
        }
        Err(failure) => {
            core.note_failure(&crate::flush::failure_text(&failure))?;
            Ok(Fetch::Failed(Pulled::Retry))
        }
    }
}

fn read_reply(core: &Core, reply: &WireReply) -> Result<Fetch, SyncError> {
    match gate(core, reply) {
        Gate::Fine => {
            let page =
                serde_json::from_slice(&reply.body).map_err(|e| SyncError::Reply(e.to_string()))?;
            Ok(Fetch::Page(page))
        }
        Gate::Halted => Ok(Fetch::Failed(Pulled::Halted)),
        Gate::Trouble => Ok(Fetch::Failed(Pulled::Retry)),
        Gate::Refused(problem) => Ok(refusal(&problem)),
    }
}

fn refusal(problem: &ProblemJson) -> Fetch {
    if problem.code == AHEAD_CODE {
        Fetch::Mismatch
    } else {
        Fetch::Failed(Pulled::Refused)
    }
}

pub(crate) async fn pull_stream(core: &Core, stream: &str) -> Result<Pulled, SyncError> {
    let mut stalls: u32 = 0;
    loop {
        let progress = core.store.progress(stream).await?;
        let page = match fetch_page(core, stream, progress.seq, &progress.record_id).await? {
            Fetch::Page(page) => page,
            Fetch::Mismatch => return Ok(Pulled::Mismatch),
            Fetch::Failed(why) => return Ok(why),
        };
        let outcome = apply(core, stream, &page.records).await?;
        stalls = if outcome.inserted > 0 {
            0
        } else {
            stalls.saturating_add(1)
        };
        if stalls >= STALLS_BEFORE_RETRY {
            core.note_server_trouble();
            return Ok(Pulled::Retry);
        }
        if !page.has_more && !outcome.gap {
            return Ok(Pulled::Caught);
        }
    }
}

async fn apply(
    core: &Core,
    stream: &str,
    records: &[StoredRecord],
) -> Result<ApplyOutcome, SyncError> {
    let outcome = core.store.apply_page(stream, records).await?;
    if outcome.inserted > 0 {
        let at = core.now_ms()?;
        core.status.update(|status| status.last_sync_ms = Some(at));
    }
    Ok(outcome)
}
