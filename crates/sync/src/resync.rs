use std::collections::HashSet;

use anybot_protocol::StoredRecord;
use anybot_store_client::{LocalRecord, NewOutbox, written_by_server};

use crate::core::Core;
use crate::error::SyncError;
use crate::pull::{Fetch, Pulled, fetch_page};
use crate::status::ResyncNotice;

const STALLS_BEFORE_RETRY: u32 = 5;
const NOTICES_KEPT: usize = 50;

enum Fetched {
    All(Vec<StoredRecord>),
    Stopped(Pulled),
}

pub(crate) async fn resync_stream(core: &Core, stream: &str) -> Result<Pulled, SyncError> {
    let server = match fetch_all(core, stream).await? {
        Fetched::All(records) => records,
        Fetched::Stopped(why) => return Ok(why),
    };
    let notice = resend_missing(core, stream, &server).await?;
    core.store.replace_stream(stream, &server).await?;
    record_notice(core, notice)?;
    core.wanted.pull(stream);
    Ok(Pulled::Caught)
}

async fn fetch_all(core: &Core, stream: &str) -> Result<Fetched, SyncError> {
    let mut all: Vec<StoredRecord> = Vec::new();
    let mut stalls: u32 = 0;
    loop {
        let (after, last_id) = all
            .last()
            .map_or((0, String::new()), |rec| (rec.seq, rec.device_id.clone()));
        let page = match fetch_page(core, stream, after, &last_id).await? {
            Fetch::Page(page) => page,
            Fetch::Mismatch => return Ok(Fetched::Stopped(Pulled::Retry)),
            Fetch::Failed(why) => return Ok(Fetched::Stopped(why)),
        };
        let before = all.len();
        let mut complete = true;
        for record in page.records {
            let wanted = i64::try_from(all.len())
                .map_err(|e| SyncError::Config(e.to_string()))?
                .saturating_add(1);
            if record.seq == wanted {
                all.push(record);
            } else if record.seq > wanted {
                complete = false;
                break;
            }
        }
        stalls = if all.len() > before {
            0
        } else {
            stalls.saturating_add(1)
        };
        if stalls >= STALLS_BEFORE_RETRY {
            core.note_server_trouble();
            return Ok(Fetched::Stopped(Pulled::Retry));
        }
        if !page.has_more && complete {
            return Ok(Fetched::All(all));
        }
    }
}

async fn resend_missing(
    core: &Core,
    stream: &str,
    server: &[StoredRecord],
) -> Result<ResyncNotice, SyncError> {
    let on_server: HashSet<&str> = server.iter().map(|rec| rec.device_id.as_str()).collect();
    let queued: HashSet<String> = core
        .store
        .outbox_all()
        .await?
        .into_iter()
        .filter(|row| row.stream == stream)
        .map(|row| row.record_id)
        .collect();
    let mut notice = ResyncNotice {
        stream: stream.to_owned(),
        at_ms: core.now_ms()?,
        resent: 0,
        server_written_not_resent: 0,
        purged_not_resent: 0,
    };
    for local in core.store.records(stream).await? {
        if on_server.contains(local.record_id.as_str()) {
            continue;
        }
        if local.purged {
            notice.purged_not_resent = notice.purged_not_resent.saturating_add(1);
        } else if written_by_server(&local.kind) {
            notice.server_written_not_resent = notice.server_written_not_resent.saturating_add(1);
        } else if !queued.contains(&local.record_id) {
            core.store.enqueue(&outbox_item(&local)).await?;
            notice.resent = notice.resent.saturating_add(1);
        }
    }
    core.refresh_counts().await?;
    core.wake.notify_one();
    Ok(notice)
}

fn outbox_item(local: &LocalRecord) -> NewOutbox {
    NewOutbox {
        stream: local.stream.clone(),
        record_id: local.record_id.clone(),
        kind: local.kind.clone(),
        content: local.content.clone(),
        created_at_ms: local.created_at_ms,
        task_id: local.task_id.clone(),
        offline_at_create: local.offline_sent,
    }
}

fn record_notice(core: &Core, notice: ResyncNotice) -> Result<(), SyncError> {
    let skipped = u64::try_from(notice.server_written_not_resent)
        .map_err(|e| SyncError::Config(e.to_string()))?;
    core.status.update(|status| {
        status.unrecovered_kinds = status.unrecovered_kinds.saturating_add(skipped);
        status.resyncs.push(notice);
        let extra = status.resyncs.len().saturating_sub(NOTICES_KEPT);
        status.resyncs.drain(..extra);
    });
    Ok(())
}
