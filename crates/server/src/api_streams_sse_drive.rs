use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use futures_util::future::{self, Ready};
use tokio::sync::broadcast;

use anybot_bus::{Notification, stream_id_of_note};
use anybot_foundation::Clock;
use anybot_protocol::{
    HEARTBEAT_INTERVAL_SECS, NewRecordsEvent, ProgressEvent, StreamProgress, sse_event_line,
    sse_heartbeat_line,
};
use anybot_store_server::{Pool, streams};

pub struct Tail {
    pub rx: broadcast::Receiver<Notification>,
    pub pool: Pool,
    pub shutdown: Arc<tokio::sync::Notify>,
    pub clock: Arc<dyn Clock>,
    pub account: String,
    pub keys: Vec<String>,
    pub init_seq: i64,
}

#[must_use]
pub fn open_stream(head: Vec<String>, tail: Tail) -> tokio::sync::mpsc::Receiver<String> {
    let (tx, rx) = tokio::sync::mpsc::channel(128);
    tokio::spawn(drive(head, tail, tx));
    rx
}

pub fn line_result(line: String) -> Ready<Result<String, std::io::Error>> {
    future::ready(Ok(line))
}

pub fn json_line<T: serde::Serialize>(name: &str, event: &T) -> Result<String, serde_json::Error> {
    let json = serde_json::to_string(event)?;
    Ok(sse_event_line(name, &json))
}

enum Step {
    End,
    Beat,
    Push(String),
    Skip,
}

async fn drive(head: Vec<String>, tail: Tail, tx: tokio::sync::mpsc::Sender<String>) {
    for line in head {
        if tx.send(line).await.is_err() {
            return;
        }
    }
    let ending = tail.shutdown.notified();
    tokio::pin!(ending);
    let mut rx = tail.rx;
    let mut last: HashMap<String, i64> = HashMap::new();
    last.insert("account/".to_owned() + &tail.account, tail.init_seq);
    let keys = tail.keys;
    let pool = tail.pool;
    let clock = tail.clock;
    let account = tail.account;
    loop {
        let step = tokio::select! {
            () = &mut ending => Step::End,
            () = clock.sleep(Duration::from_secs(HEARTBEAT_INTERVAL_SECS)) => Step::Beat,
            note = rx.recv() => on_note(&account, &keys, &pool, &mut last, note).await,
        };
        match step {
            Step::End => break,
            Step::Beat => {
                if tx.send(sse_heartbeat_line().to_owned()).await.is_err() {
                    return;
                }
            }
            Step::Push(line) => {
                if tx.send(line).await.is_err() {
                    return;
                }
            }
            Step::Skip => {}
        }
    }
    send_closing(&keys, &pool, &last, &tx).await;
}

async fn on_note(
    account: &str,
    keys: &[String],
    pool: &Pool,
    last: &mut HashMap<String, i64>,
    note: Result<Notification, broadcast::error::RecvError>,
) -> Step {
    let Ok(note) = note else {
        return Step::Skip;
    };
    if anybot_bus::account_of_revoke(&note.kind) == Some(account) {
        return Step::End;
    }
    let Some(key) = stream_id_of_note(&note.kind) else {
        return Step::Skip;
    };
    if !keys.iter().any(|k| k == key) {
        return Step::Skip;
    }
    let Ok(top) = streams::max_seq(pool, key).await else {
        return Step::Skip;
    };
    last.insert(key.to_owned(), top);
    let device_id = closing_device(pool, key, top).await;
    let event = NewRecordsEvent {
        stream: key.to_owned(),
        seq: top,
        device_id,
    };
    let Ok(line) = json_line("new-records", &event) else {
        return Step::Skip;
    };
    Step::Push(line)
}

async fn send_closing(
    keys: &[String],
    pool: &Pool,
    last: &HashMap<String, i64>,
    tx: &tokio::sync::mpsc::Sender<String>,
) {
    let mut rows = Vec::new();
    for key in keys {
        let seq = match last.get(key) {
            Some(seq) => *seq,
            None => streams::max_seq(pool, key).await.unwrap_or(0),
        };
        rows.push(StreamProgress {
            stream: key.clone(),
            seq,
            device_id: closing_device(pool, key, seq).await,
        });
    }
    let event = ProgressEvent { streams: rows };
    match json_line("stream-progress", &event) {
        Ok(line) => {
            let _sent = tx.send(line).await;
        }
        Err(e) => tracing::warn!("sse closing encode failed: {e}"),
    }
}

async fn closing_device(pool: &Pool, key: &str, seq: i64) -> String {
    match streams::record_at(pool, key, seq).await {
        Ok(Some(row)) => row.device_id,
        Ok(None) => String::new(),
        Err(e) => {
            tracing::warn!("sse record_at failed: {e}");
            String::new()
        }
    }
}
