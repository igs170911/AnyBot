use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use serde_json::{Value, json};
use test_harness::FakeClock;

use crate::wire::{Wire, WireFailure, WireFuture, WireMethod, WireReply, WireRequest};

pub(crate) type Scripted = Result<WireReply, WireFailure>;

struct GetScript {
    path_has: String,
    replies: VecDeque<Scripted>,
}

pub(crate) struct FakeWire {
    clock: Arc<FakeClock>,
    calls: Mutex<Vec<u64>>,
    posted: Mutex<Vec<Value>>,
    gets: Mutex<Vec<String>>,
    script: Mutex<VecDeque<Scripted>>,
    get_scripts: Mutex<Vec<GetScript>>,
    fail_forever: bool,
}

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl FakeWire {
    fn build(clock: &Arc<FakeClock>, script: Vec<Scripted>, fail_forever: bool) -> Arc<FakeWire> {
        Arc::new(FakeWire {
            clock: Arc::clone(clock),
            calls: Mutex::new(Vec::new()),
            posted: Mutex::new(Vec::new()),
            gets: Mutex::new(Vec::new()),
            script: Mutex::new(script.into()),
            get_scripts: Mutex::new(Vec::new()),
            fail_forever,
        })
    }

    pub(crate) fn new(clock: &Arc<FakeClock>, script: Vec<Scripted>) -> Arc<FakeWire> {
        FakeWire::build(clock, script, false)
    }

    pub(crate) fn always_failing(clock: &Arc<FakeClock>) -> Arc<FakeWire> {
        FakeWire::build(clock, Vec::new(), true)
    }

    pub(crate) fn on_get(&self, path_has: &str, replies: Vec<Scripted>) {
        locked(&self.get_scripts).push(GetScript {
            path_has: path_has.to_owned(),
            replies: replies.into(),
        });
    }

    pub(crate) fn calls(&self) -> Vec<u64> {
        locked(&self.calls).clone()
    }

    pub(crate) fn posted(&self) -> Vec<Value> {
        locked(&self.posted).clone()
    }

    pub(crate) fn gets(&self) -> Vec<String> {
        locked(&self.gets).clone()
    }

    pub(crate) fn gaps(&self) -> Vec<u64> {
        let times = self.calls();
        times
            .iter()
            .zip(times.iter().skip(1))
            .map(|(before, after)| after.saturating_sub(*before))
            .collect()
    }

    fn answer_get(&self, path: &str) -> Scripted {
        locked(&self.gets).push(path.to_owned());
        let mut scripts = locked(&self.get_scripts);
        let scripted = scripts
            .iter_mut()
            .find(|entry| path.contains(&entry.path_has))
            .and_then(|entry| entry.replies.pop_front());
        scripted.unwrap_or_else(|| Ok(page_reply("account", &[], false, 0)))
    }
}

pub(crate) fn page_reply(
    stream: &str,
    records: &[Value],
    has_more: bool,
    max_seq: i64,
) -> WireReply {
    let selector = if stream == "account" {
        json!("account")
    } else {
        json!({"bot": {"id": stream}})
    };
    let body = json!({
        "stream": selector,
        "records": records,
        "has_more": has_more,
        "max_seq": max_seq,
    });
    WireReply {
        status: 200,
        body: body.to_string().into_bytes(),
        server_time_ms: None,
    }
}

pub(crate) fn stored(seq: i64, id: &str, kind: &str, text: &str) -> Value {
    json!({
        "seq": seq,
        "device_id": id,
        "kind": kind,
        "content": {"text": text},
        "created_at_ms": 1_700_000_000_000_i64,
        "offline_sent": false,
    })
}

fn accepted_for(body: &Value) -> WireReply {
    let items = body
        .get("items")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let results: Vec<Value> = (1..=items)
        .map(|seq| json!({"result": "accepted", "seq": seq}))
        .collect();
    let reply = json!({"stream": body.get("stream"), "results": results});
    WireReply {
        status: 200,
        body: reply.to_string().into_bytes(),
        server_time_ms: None,
    }
}

pub(crate) fn problem(status: u16, code: &str) -> WireReply {
    let body = json!({
        "type": "https://anybot.test/problems/x",
        "title": "x",
        "status": status,
        "code": code,
        "params": {},
    });
    WireReply {
        status,
        body: body.to_string().into_bytes(),
        server_time_ms: None,
    }
}

impl Wire for FakeWire {
    fn call(&self, request: WireRequest) -> WireFuture<'_> {
        if request.method == WireMethod::Get {
            let answer = self.answer_get(&request.path);
            return Box::pin(async move { answer });
        }
        let body: Value = request
            .body
            .as_deref()
            .and_then(|text| serde_json::from_str(text).ok())
            .unwrap_or(Value::Null);
        locked(&self.calls).push(self.clock.now_ms());
        locked(&self.posted).push(body.clone());
        let scripted = locked(&self.script).pop_front();
        let answer = match scripted {
            Some(step) => step,
            None if self.fail_forever => Err(WireFailure::Network("down".to_owned())),
            None => Ok(accepted_for(&body)),
        };
        Box::pin(async move { answer })
    }
}
