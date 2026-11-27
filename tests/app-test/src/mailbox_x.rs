use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::json;
use tokio::task::JoinHandle;

use crate::http::{ConnectionPool, RequestSpec};
use crate::sse::{SseOpen, data_json, event_name, open_sse};

const IDLE: Duration = Duration::from_secs(30);
const STREAM_ENDED: &str = "sse stream ended";

#[derive(Default, Clone)]
pub struct XLog {
    pub claimed: Vec<String>,
    pub refused: Vec<String>,
    pub failures: Vec<String>,
}

pub struct XStandIn {
    pub log: Arc<Mutex<XLog>>,
    task: JoinHandle<()>,
}

impl XStandIn {
    pub async fn start(base: &str, bearer: &str) -> Result<Self, String> {
        let stream = open_sse(base, "/v1/streams/notifications", bearer, "").await?;
        let log = Arc::new(Mutex::new(XLog::default()));
        let worker = Worker {
            base: base.to_owned(),
            bearer: bearer.to_owned(),
            http: ConnectionPool::new(4)?,
            log: Arc::clone(&log),
            keys: 0,
            seen: HashSet::new(),
        };
        let task = tokio::spawn(worker.run(stream));
        Ok(XStandIn { log, task })
    }

    pub fn claimed_count(&self) -> Result<usize, String> {
        let log = self.log.lock().map_err(|e| e.to_string())?;
        Ok(log.claimed.len())
    }

    pub fn claimed_at(&self, pick: u64) -> Result<Option<String>, String> {
        let log = self.log.lock().map_err(|e| e.to_string())?;
        let Ok(len) = u64::try_from(log.claimed.len()) else {
            return Err("claimed list too long".to_owned());
        };
        let Some(at) = pick.checked_rem(len) else {
            return Ok(None);
        };
        let at = usize::try_from(at).map_err(|e| e.to_string())?;
        Ok(log.claimed.get(at).cloned())
    }

    pub fn snapshot(&self) -> Result<XLog, String> {
        let log = self.log.lock().map_err(|e| e.to_string())?;
        Ok(log.clone())
    }

    pub fn stop(&self) {
        self.task.abort();
    }
}

struct Worker {
    base: String,
    bearer: String,
    http: ConnectionPool,
    log: Arc<Mutex<XLog>>,
    keys: u64,
    seen: HashSet<String>,
}

impl Worker {
    async fn run(mut self, first: SseOpen) {
        let mut stream = first;
        loop {
            self.drain(&mut stream).await;
            stream = match open_sse(&self.base, "/v1/streams/notifications", &self.bearer, "").await
            {
                Ok(reopened) => reopened,
                Err(reason) => {
                    self.note(|log| log.failures.push(reason));
                    return;
                }
            };
        }
    }

    async fn drain(&mut self, stream: &mut SseOpen) {
        loop {
            match stream.recv_within(IDLE).await {
                Ok(block) => self.on_block(&block).await,
                Err(reason) if reason == STREAM_ENDED => return,
                Err(_idle) => {}
            }
        }
    }

    fn note(&self, change: impl FnOnce(&mut XLog)) {
        if let Ok(mut log) = self.log.lock() {
            change(&mut log);
        }
    }

    async fn on_block(&mut self, block: &str) {
        if event_name(block).as_deref() != Some("mail") {
            return;
        }
        let hint = match data_json(block) {
            Ok(hint) => hint,
            Err(reason) => return self.note(|log| log.failures.push(reason)),
        };
        let Some(letter) = hint.get("letter_id").and_then(|v| v.as_str()) else {
            return self.note(|log| log.failures.push(format!("hint without letter: {hint}")));
        };
        if self.seen.insert(letter.to_owned()) {
            self.claim(letter).await;
        }
    }

    async fn claim(&mut self, letter: &str) {
        self.keys = self.keys.saturating_add(1);
        let key = format!("x-claim-{}", self.keys);
        let spec = RequestSpec::new(&json!({"letter_id": letter}).to_string())
            .with_key(&key)
            .with_bearer(&self.bearer);
        match self.http.post(&self.base, "/v1/mailbox/claim", &spec).await {
            Ok(answer) if answer.status == 200 => {
                self.note(|log| log.claimed.push(letter.to_owned()));
            }
            Ok(answer) => {
                self.note(|log| log.refused.push(format!("{letter}: {}", answer.json)));
            }
            Err(reason) => self.note(|log| log.failures.push(reason)),
        }
    }
}
