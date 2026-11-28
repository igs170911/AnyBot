use std::collections::{BTreeMap, BTreeSet};

use anybot_foundation::TaskId;
use anybot_protocol::{KnownTaskEventKind, TaskEvent, TaskEventKind, ToolEffect};
use anybot_store_client::{JournalEntry, JournalRow, LocalRecord, OUTBOX_SEQ_BASE, bot_stream};
use serde_json::{Value, json};

use crate::journal_mask::{SecretMasker, mask_json};
use crate::record::{
    NewTaskEvent, RecordEntry, RecordError, Stored, StoredBody, UserLine, task_event_of,
};

pub(crate) enum JournalPayload {
    UserSaid(UserLine),
    Task(NewTaskEvent),
}

pub(crate) fn build_entry(
    entry: &RecordEntry,
    masker: &dyn SecretMasker,
) -> Result<JournalEntry, RecordError> {
    let raw_value = encode_entry(entry)?;
    let local_detail = serde_json::to_string(&raw_value).map_err(|e| serde_err(&e))?;
    let content = mask_json(raw_value, masker);
    Ok(match entry {
        RecordEntry::UserSaid(line) => JournalEntry {
            stream: bot_stream(&line.bot.as_str()),
            record_id: line.record_id.as_str(),
            kind: "user_said".to_owned(),
            content,
            local_detail: Some(local_detail),
            created_at_ms: timestamp_i64(line.at_ms),
            task_id: Some(line.task.as_str()),
            turn: None,
            effect: None,
            offline_at_create: false,
        },
        RecordEntry::Task(new) => JournalEntry {
            stream: bot_stream(&new.bot.as_str()),
            record_id: new.event_id.as_str(),
            kind: new.kind.kind_name(),
            content,
            local_detail: Some(local_detail),
            created_at_ms: timestamp_i64(new.at_ms),
            task_id: Some(new.task.as_str()),
            turn: new.turn.map(i64::from),
            effect: intent_effect(&new.kind),
            offline_at_create: false,
        },
    })
}

fn encode_entry(entry: &RecordEntry) -> Result<Value, RecordError> {
    match entry {
        RecordEntry::UserSaid(line) => Ok(json!({
            "entry": "user_said",
            "record_id": line.record_id,
            "bot": line.bot,
            "task": line.task,
            "device": line.device,
            "text": line.text,
            "at_ms": line.at_ms,
        })),
        RecordEntry::Task(new) => Ok(json!({
            "entry": "task",
            "bot": new.bot,
            "task": new.task,
            "turn": new.turn,
            "event_id": new.event_id,
            "at_ms": new.at_ms,
            "kind": serde_json::to_value(&new.kind).map_err(|e| serde_err(&e))?,
        })),
    }
}

fn decode_value(value: &Value) -> Result<JournalPayload, RecordError> {
    let kind = value
        .get("entry")
        .and_then(Value::as_str)
        .ok_or_else(|| missing("entry"))?;
    match kind {
        "user_said" => Ok(JournalPayload::UserSaid(decode_user_line(value)?)),
        "task" => Ok(JournalPayload::Task(decode_task_event(value)?)),
        other => Err(RecordError {
            reason: format!("unknown journal entry tag {other}"),
        }),
    }
}

fn decode_user_line(value: &Value) -> Result<UserLine, RecordError> {
    let text = value
        .get("text")
        .and_then(Value::as_str)
        .ok_or_else(|| missing("text"))?
        .to_owned();
    Ok(UserLine {
        record_id: serde_json::from_value(raw(value, "record_id")?).map_err(|e| serde_err(&e))?,
        bot: serde_json::from_value(raw(value, "bot")?).map_err(|e| serde_err(&e))?,
        task: serde_json::from_value(raw(value, "task")?).map_err(|e| serde_err(&e))?,
        device: serde_json::from_value(raw(value, "device")?).map_err(|e| serde_err(&e))?,
        text,
        at_ms: serde_json::from_value(raw(value, "at_ms")?).map_err(|e| serde_err(&e))?,
    })
}

fn decode_task_event(value: &Value) -> Result<NewTaskEvent, RecordError> {
    Ok(NewTaskEvent {
        bot: serde_json::from_value(raw(value, "bot")?).map_err(|e| serde_err(&e))?,
        task: serde_json::from_value(raw(value, "task")?).map_err(|e| serde_err(&e))?,
        turn: serde_json::from_value(raw(value, "turn")?).map_err(|e| serde_err(&e))?,
        event_id: serde_json::from_value(raw(value, "event_id")?).map_err(|e| serde_err(&e))?,
        at_ms: serde_json::from_value(raw(value, "at_ms")?).map_err(|e| serde_err(&e))?,
        kind: serde_json::from_value(raw(value, "kind")?).map_err(|e| serde_err(&e))?,
    })
}

fn raw(value: &Value, key: &str) -> Result<Value, RecordError> {
    value.get(key).cloned().ok_or_else(|| missing(key))
}

fn missing(key: &str) -> RecordError {
    RecordError {
        reason: format!("missing journal field {key}"),
    }
}

fn intent_effect(kind: &TaskEventKind) -> Option<ToolEffect> {
    if let TaskEventKind::Known(KnownTaskEventKind::ToolIntent { level, .. }) = kind {
        Some(*level)
    } else {
        None
    }
}

pub(crate) fn merge_conversation(
    replica: &[LocalRecord],
    pending: &[JournalRow],
) -> Result<Vec<Stored>, RecordError> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    let mut seqs: BTreeMap<TaskId, u64> = BTreeMap::new();
    for rec in replica {
        if let Ok(payload) = decode_value(&rec.content) {
            seen.insert(rec.record_id.clone());
            let stream_seq = u64::try_from(rec.seq).unwrap_or(0);
            out.push(stored_from_payload(stream_seq, payload, &mut seqs));
        }
    }
    for row in pending {
        if seen.contains(&row.record_id) {
            continue;
        }
        let payload = decode_row(row)?;
        let stream_seq = outbox_stream_seq(row.local_id)?;
        out.push(stored_from_payload(stream_seq, payload, &mut seqs));
    }
    Ok(out)
}

pub(crate) fn merge_task_events(
    task: TaskId,
    replica: &[LocalRecord],
    pending: &[JournalRow],
) -> Result<Vec<TaskEvent>, RecordError> {
    let mut events = Vec::new();
    let mut seen = BTreeSet::new();
    let task_str = task.as_str();
    for rec in replica {
        if rec.task_id.as_deref() != Some(task_str.as_str()) {
            continue;
        }
        if let Ok(JournalPayload::Task(new)) = decode_value(&rec.content) {
            seen.insert(rec.record_id.clone());
            let seq = u64::try_from(events.len().saturating_add(1)).unwrap_or(u64::MAX);
            events.push(task_event_of(&new, seq));
        }
    }
    for row in pending {
        if seen.contains(&row.record_id) {
            continue;
        }
        if let JournalPayload::Task(new) = decode_row(row)?
            && new.task == task
        {
            let seq = u64::try_from(events.len().saturating_add(1)).unwrap_or(u64::MAX);
            events.push(task_event_of(&new, seq));
        }
    }
    Ok(events)
}

fn stored_from_payload(
    stream_seq: u64,
    payload: JournalPayload,
    seqs: &mut BTreeMap<TaskId, u64>,
) -> Stored {
    let body = match payload {
        JournalPayload::UserSaid(line) => StoredBody::UserSaid(line),
        JournalPayload::Task(new) => {
            let next = seqs.get(&new.task).map_or(1, |v| v.saturating_add(1));
            seqs.insert(new.task, next);
            StoredBody::Task {
                task: new.task,
                turn: new.turn,
                event: task_event_of(&new, next),
            }
        }
    };
    Stored { stream_seq, body }
}

fn decode_row(row: &JournalRow) -> Result<JournalPayload, RecordError> {
    if let Some(raw_text) = &row.local_detail {
        let val: Value = serde_json::from_str(raw_text).map_err(|e| serde_err(&e))?;
        return decode_value(&val);
    }
    decode_value(&row.content)
}

pub(crate) fn outbox_stream_seq(local_id: i64) -> Result<u64, RecordError> {
    let combined = OUTBOX_SEQ_BASE.saturating_add(local_id);
    u64::try_from(combined).map_err(|e| RecordError {
        reason: e.to_string(),
    })
}

fn timestamp_i64(ts: anybot_foundation::TimestampMs) -> i64 {
    i64::try_from(ts.millis()).unwrap_or(i64::MAX)
}

fn serde_err(err: &serde_json::Error) -> RecordError {
    RecordError {
        reason: err.to_string(),
    }
}
