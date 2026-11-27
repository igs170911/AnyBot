use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use anybot_app_test::clockwalk::ClockWalker;
use anybot_app_test::http::{RequestSpec, get, post};
use anybot_sync::reconnect_status::{LinkPhase, LinkStatus};
use anybot_sync::{NewRecord, SyncClient};
use serde_json::{Value, json};
use test_harness::{FakeClock, sleep_real};
use tokio::task::JoinHandle;

use crate::common::Rig;
use crate::scene::bot;

pub(crate) const STEP_KINDS: [&str; 4] = [
    "context_built",
    "model_asked",
    "model_answer",
    "model_usage",
];

pub(crate) fn phase(client: &SyncClient) -> Option<LinkPhase> {
    client.status().link.map(|link| link.phase)
}

pub(crate) async fn drive_to_phase(
    client: &SyncClient,
    clock: &Arc<FakeClock>,
    step: Duration,
    want: LinkPhase,
) -> Result<(), String> {
    for _round in 0..3000 {
        if phase(client) == Some(want) {
            return Ok(());
        }
        sleep_real(Duration::from_millis(10)).await;
        let _woken = clock.advance(step);
    }
    Err(format!(
        "the link never reached {want:?}; last {:?}",
        phase(client)
    ))
}

pub(crate) struct PhaseLog {
    rows: Arc<Mutex<Vec<LinkStatus>>>,
    task: JoinHandle<()>,
}

impl PhaseLog {
    pub(crate) fn start(client: &SyncClient) -> PhaseLog {
        let rows = Arc::new(Mutex::new(Vec::new()));
        let log = Arc::clone(&rows);
        let mut watcher = client.watch_status();
        let task = tokio::spawn(async move {
            loop {
                let seen = watcher.borrow_and_update().link;
                if let Some(link) = seen {
                    let mut kept = log.lock().unwrap_or_else(PoisonError::into_inner);
                    if kept.last() != Some(&link) {
                        kept.push(link);
                    }
                }
                if watcher.changed().await.is_err() {
                    return;
                }
            }
        });
        PhaseLog { rows, task }
    }

    pub(crate) fn rows(&self) -> Vec<LinkStatus> {
        self.rows
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Drop for PhaseLog {
    fn drop(&mut self) {
        self.task.abort();
    }
}

pub(crate) async fn phone_login(rig: &Rig) -> Result<String, String> {
    device_login(rig, "phone-p").await
}

pub(crate) async fn device_login(rig: &Rig, label: &str) -> Result<String, String> {
    let body = json!({
        "account": "alice",
        "password": "alicepassword",
        "label": label,
        "credential": "bearer",
    })
    .to_string();
    let spec = RequestSpec::new(&body).with_key(&format!("k-sync-login-{label}"));
    let mut walker = ClockWalker::start(&rig.served.clock);
    let answer = post(&rig.served.base_url(), "/v1/account/login", &spec).await;
    walker.stop().await?;
    answer?.bearer()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Seen {
    pub(crate) seq: i64,
    pub(crate) record_id: String,
    pub(crate) kind: String,
}

pub(crate) async fn phone_reads(rig: &Rig, bearer: &str) -> Result<Vec<Seen>, String> {
    let spec = RequestSpec::new("").with_bearer(bearer);
    let path = "/v1/streams/after?stream=bot%2Fbot-1&after=0";
    let answer = get(&rig.served.base_url(), path, &spec).await?;
    if answer.status != 200 {
        return Err(format!("phone read: {} {}", answer.status, answer.json));
    }
    let records = answer
        .json
        .get("records")
        .and_then(Value::as_array)
        .ok_or(format!("no records in {}", answer.json))?;
    records.iter().map(seen_of).collect()
}

fn seen_of(record: &Value) -> Result<Seen, String> {
    let text = |key: &str| {
        record
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or(format!("no {key} in {record}"))
    };
    Ok(Seen {
        seq: record
            .get("seq")
            .and_then(Value::as_i64)
            .ok_or(format!("no seq in {record}"))?,
        record_id: text("device_id")?,
        kind: text("kind")?,
    })
}

pub(crate) async fn write_steps(
    client: &SyncClient,
    steps: usize,
    tag: &str,
) -> Result<Vec<String>, String> {
    let mut ids = Vec::new();
    for step in 0..steps {
        for kind in STEP_KINDS {
            let record = NewRecord {
                stream: bot(),
                kind: kind.to_owned(),
                content: json!({"step": step, "by": tag}),
                task_id: Some(format!("task-{tag}")),
            };
            ids.push(client.add_record(record).await.map_err(|e| e.to_string())?);
        }
    }
    Ok(ids)
}

pub(crate) fn numbered_from_one(seen: &[Seen]) -> bool {
    seen.iter()
        .zip(1_i64..)
        .all(|(row, expected)| row.seq == expected)
}
