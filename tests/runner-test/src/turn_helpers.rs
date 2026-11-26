use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use anybot_foundation::TaskId;
use anybot_messages::{Catalog, Lang, Params};
use anybot_protocol::{KnownTaskEventKind, TaskDetail, TaskStatus};
use anybot_runner::StoredBody;
use sha2::{Digest, Sha256};
use std::borrow::ToOwned;
use test_harness::{
    FakeClock, FakeModel, RecordedRequest, advance_clock, sleep_real, timeout_real,
};

use crate::mem_store::{MemoryRaw, MemoryRecord};
use crate::turn::{Device, TurnWorld};

#[must_use]
pub fn task_kinds(world: &TurnWorld, task: &TaskId) -> Vec<KnownTaskEventKind> {
    world.kinds(*task)
}

#[must_use]
pub fn kind_names(world: &TurnWorld, task: &TaskId) -> Vec<String> {
    world.kind_names(*task)
}

#[must_use]
pub fn chat_requests(model: &FakeModel) -> Vec<RecordedRequest> {
    model.recorded()
}

pub fn request_body(model: &FakeModel, index: usize) -> Result<serde_json::Value, String> {
    let recorded = model.recorded();
    let Some(record) = recorded.get(index) else {
        return Err(format!("no request at index {index}"));
    };
    serde_json::from_str(&record.body).map_err(|e| format!("{e}"))
}

pub fn system_content(body: &serde_json::Value) -> Result<String, String> {
    let Some(first) = body.get("messages").and_then(|m| m.get(0)) else {
        return Err("no messages".to_owned());
    };
    let Some(content) = first.get("content").and_then(|c| c.as_str()) else {
        return Err("no system content".to_owned());
    };
    Ok(content.to_owned())
}

#[must_use]
pub fn messages_of(body: &serde_json::Value) -> Vec<serde_json::Value> {
    match body.get("messages") {
        Some(serde_json::Value::Array(items)) => items.clone(),
        Some(_) | None => Vec::new(),
    }
}

#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let out = hasher.finalize();
    let mut text = String::new();
    for b in &out {
        match std::fmt::Write::write_fmt(&mut text, format_args!("{b:02x}")) {
            Ok(()) => {}
            Err(_e) => return text,
        }
    }
    text
}

#[must_use]
pub fn raw_has(raw: &MemoryRaw, id: &str) -> bool {
    raw.items().iter().any(|(digest, _v)| digest.hex() == id)
}

#[must_use]
pub fn raw_bytes_of(raw: &MemoryRaw, id: &str) -> Option<Vec<u8>> {
    for (digest, value) in raw.items() {
        if digest.hex() == id {
            return Some(value);
        }
    }
    None
}

#[must_use]
pub fn raw_items_len(raw: &MemoryRaw) -> usize {
    raw.items().len()
}

pub fn state_of(
    world: &TurnWorld,
    task: &TaskId,
) -> Result<(TaskStatus, Option<TaskDetail>), String> {
    world.task_state(*task)
}

#[must_use]
pub fn probe_of(record: &MemoryRecord, task: &TaskId, name: &str, index: usize) -> Option<usize> {
    let mut seen = 0usize;
    for (row, probe) in record.entries() {
        let StoredBody::Task {
            task: owner, event, ..
        } = &row.body
        else {
            continue;
        };
        if *owner != *task {
            continue;
        }
        let Some(kind) = known_kind(event) else {
            continue;
        };
        if kind.kind_name() != name {
            continue;
        }
        if seen == index {
            return Some(probe);
        }
        seen = seen.checked_add(1).unwrap_or(0);
    }
    None
}

fn known_kind(event: &anybot_protocol::TaskEvent) -> Option<KnownTaskEventKind> {
    match &event.kind {
        anybot_protocol::TaskEventKind::Known(kind) => Some(kind.clone()),
        anybot_protocol::TaskEventKind::Raw(_r) => None,
    }
}

pub fn fmt_text(lang: Lang, code: &str, params: &Params) -> Result<String, String> {
    Catalog::new()
        .format_str(lang, code, params)
        .map_err(|e| format!("{e:?}"))
}

pub async fn drive<F>(clock: &Arc<FakeClock>, future: F) -> Result<F::Output, String>
where
    F: Future,
{
    let push = tokio::spawn(push_clock(Arc::clone(clock)));
    let result = timeout_real(future, Duration::from_secs(10)).await;
    push.abort();
    result.map_err(|e| format!("drive timed out: {e}"))
}

async fn push_clock(clock: Arc<FakeClock>) {
    for _step in 0..10_000 {
        advance_clock(&clock, Duration::from_secs(1));
        sleep_real(Duration::from_millis(10)).await;
    }
}

#[must_use]
pub fn device_of(world: &TurnWorld, which: Device) -> DeviceIdAlias {
    world.device(which)
}

type DeviceIdAlias = anybot_foundation::DeviceId;

#[must_use]
pub fn task_of(outcome: &anybot_runner::TurnOutcome) -> TaskId {
    outcome.task
}

#[must_use]
pub fn record_len(record: &MemoryRecord) -> usize {
    record.len()
}
