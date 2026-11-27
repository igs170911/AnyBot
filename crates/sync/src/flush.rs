use anybot_protocol::{
    AppendReply, AppendRequest, MAX_APPEND_ITEMS, ProblemJson, RecordItem, StreamSelector,
};
use anybot_store_client::{ACCOUNT_STREAM, OutboxRow};

use crate::core::Core;
use crate::error::SyncError;
use crate::idem::append_key;
use crate::status::ConnectionState;
use crate::wire::{WireFailure, WireLimits, WireMethod, WireReply, WireRequest};

pub(crate) const BOT_CREATE_KIND: &str = "bots_create";
const VERSION_OLD_CODE: &str = "install-version-old";
const SEVEN_DAYS_MS: i64 = 7 * 24 * 60 * 60 * 1000;
const OFFLINE_AFTER_MS: i64 = 5_000;
const MIN_GAP_MS: i64 = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Flush {
    Idle,
    Sent,
    Halted,
    Retry,
}

pub(crate) async fn flush_once(core: &Core) -> Result<Flush, SyncError> {
    let now = core.now_ms()?;
    core.store
        .outbox_drop_expired(now.saturating_sub(SEVEN_DAYS_MS))
        .await?;
    if next_batch(core).await?.is_none() {
        core.refresh_counts().await?;
        return Ok(Flush::Idle);
    }
    wait_pace(core).await?;
    let Some((stream, rows)) = next_batch(core).await? else {
        core.refresh_counts().await?;
        return Ok(Flush::Sent);
    };
    let outcome = send_batch(core, &stream, rows).await?;
    core.refresh_counts().await?;
    Ok(outcome)
}

async fn next_batch(core: &Core) -> Result<Option<(String, Vec<OutboxRow>)>, SyncError> {
    let mut streams = core.store.outbox_streams().await?;
    streams.sort_by_key(|name| (name != ACCOUNT_STREAM, name.clone()));
    let limit = i64::try_from(MAX_APPEND_ITEMS).map_err(|e| SyncError::Config(e.to_string()))?;
    for stream in streams {
        if waits_for_bot_create(core, &stream).await? {
            continue;
        }
        let rows = core.store.outbox_pending(&stream, limit).await?;
        if !rows.is_empty() {
            return Ok(Some((stream, rows)));
        }
    }
    Ok(None)
}

async fn waits_for_bot_create(core: &Core, stream: &str) -> Result<bool, SyncError> {
    let Some(bot_id) = stream.strip_prefix("bot/") else {
        return Ok(false);
    };
    let limit = i64::try_from(MAX_APPEND_ITEMS).map_err(|e| SyncError::Config(e.to_string()))?;
    let account = core.store.outbox_pending(ACCOUNT_STREAM, limit).await?;
    Ok(account.iter().any(|row| {
        row.kind == BOT_CREATE_KIND
            && row
                .content
                .get("bot_id")
                .and_then(serde_json::Value::as_str)
                == Some(bot_id)
    }))
}

pub(crate) fn selector_of(stream: &str) -> Result<StreamSelector, SyncError> {
    if stream == ACCOUNT_STREAM {
        return Ok(StreamSelector::Account);
    }
    stream
        .strip_prefix("bot/")
        .map(|id| StreamSelector::Bot { id: id.to_owned() })
        .ok_or_else(|| SyncError::Config(format!("unknown stream {stream}")))
}

async fn send_batch(core: &Core, stream: &str, rows: Vec<OutboxRow>) -> Result<Flush, SyncError> {
    let (rows, fresh) = mark_attempts(core, rows).await?;
    if rows.is_empty() {
        return Ok(Flush::Sent);
    }
    let ids: Vec<&str> = rows.iter().map(|row| row.record_id.as_str()).collect();
    let body = serde_json::to_string(&AppendRequest {
        stream: selector_of(stream)?,
        items: items_of(core, &rows)?,
    })
    .map_err(|e| SyncError::Encode(e.to_string()))?;
    let request = WireRequest {
        method: WireMethod::Post,
        path: "/v1/streams/append".to_owned(),
        body: Some(body),
        idempotency_key: Some(append_key(stream, &ids)),
        limits: WireLimits::BACKGROUND,
    };
    match core.wire.call(request).await {
        Ok(reply) => {
            core.note_reply(&reply)?;
            settle(core, &rows, &reply).await
        }
        Err(failure) => {
            if matches!(
                failure,
                WireFailure::ConnectTimeout | WireFailure::ConnectFailed(_)
            ) {
                for local_id in fresh {
                    core.store.outbox_clear_attempt(local_id).await?;
                }
            }
            core.note_failure(&failure_text(&failure))?;
            Ok(Flush::Retry)
        }
    }
}

pub(crate) fn failure_text(failure: &WireFailure) -> String {
    match failure {
        WireFailure::ConnectTimeout => "connect-timeout".to_owned(),
        WireFailure::WaitTimeout => "wait-timeout".to_owned(),
        WireFailure::ConnectFailed(text) | WireFailure::Network(text) => text.clone(),
    }
}

pub(crate) async fn wait_pace(core: &Core) -> Result<(), SyncError> {
    let last = core.pace().last_send_ms;
    if let Some(last) = last {
        let waited = core.now_ms()?.saturating_sub(last);
        if waited < MIN_GAP_MS {
            let remaining = u64::try_from(MIN_GAP_MS.saturating_sub(waited))
                .map_err(|e| SyncError::Config(e.to_string()))?;
            core.clock
                .sleep(std::time::Duration::from_millis(remaining))
                .await;
        }
    }
    let at = core.now_ms()?;
    core.pace().last_send_ms = Some(at);
    Ok(())
}

async fn mark_attempts(
    core: &Core,
    rows: Vec<OutboxRow>,
) -> Result<(Vec<OutboxRow>, Vec<i64>), SyncError> {
    let now = core.now_ms()?;
    let mut kept = Vec::with_capacity(rows.len());
    let mut fresh = Vec::new();
    for row in rows {
        let offline_sent = offline_flag(&row, now);
        let present = core
            .store
            .outbox_mark_attempt(row.local_id, now, offline_sent, &row.content)
            .await?;
        if !present {
            continue;
        }
        if row.attempted_at_ms.is_none() {
            fresh.push(row.local_id);
        }
        kept.push(row);
    }
    Ok((kept, fresh))
}

fn offline_flag(row: &OutboxRow, now: i64) -> bool {
    row.offline_at_create && now.saturating_sub(row.created_at_ms) > OFFLINE_AFTER_MS
}

fn items_of(core: &Core, rows: &[OutboxRow]) -> Result<Vec<RecordItem>, SyncError> {
    let now = core.now_ms()?;
    Ok(rows
        .iter()
        .map(|row| RecordItem {
            device_id: row.record_id.clone(),
            kind: row.kind.clone(),
            content: row.content.clone(),
            created_at_ms: row.created_at_ms,
            offline_sent: offline_flag(row, now),
            task_id: row.task_id.clone(),
        })
        .collect())
}

async fn settle(core: &Core, rows: &[OutboxRow], reply: &WireReply) -> Result<Flush, SyncError> {
    if reply.status == 200 {
        return settle_results(core, rows, reply).await;
    }
    if reply.status == 401 {
        core.halt(ConnectionState::NeedLogin);
        return Ok(Flush::Halted);
    }
    let Ok(problem) = serde_json::from_slice::<ProblemJson>(&reply.body) else {
        core.note_server_trouble();
        return Ok(Flush::Retry);
    };
    if problem.code == VERSION_OLD_CODE {
        core.halt(ConnectionState::VersionTooOld);
        return Ok(Flush::Halted);
    }
    let refused = (400..500).contains(&reply.status) && reply.status != 408 && reply.status != 429;
    let head = rows.first();
    if let (true, Some(row)) = (refused, head) {
        core.store
            .outbox_reject(row.local_id, &problem.code, core.now_ms()?)
            .await?;
        Ok(Flush::Sent)
    } else {
        core.note_server_trouble();
        Ok(Flush::Retry)
    }
}

async fn settle_results(
    core: &Core,
    rows: &[OutboxRow],
    reply: &WireReply,
) -> Result<Flush, SyncError> {
    let parsed: AppendReply =
        serde_json::from_slice(&reply.body).map_err(|e| SyncError::Reply(e.to_string()))?;
    if parsed.results.len() != rows.len() {
        return Err(SyncError::Reply(
            "append reply has the wrong number of results".to_owned(),
        ));
    }
    let now = core.now_ms()?;
    for (row, result) in rows.iter().zip(parsed.results.iter()) {
        match (result.result.as_str(), result.code.as_deref()) {
            ("accepted" | "duplicate", _) => core.store.outbox_accept(row.local_id).await?,
            ("rejected", Some(why)) => core.store.outbox_reject(row.local_id, why, now).await?,
            (other, _) => {
                return Err(SyncError::Reply(format!("unknown append result {other}")));
            }
        }
    }
    Ok(Flush::Sent)
}
