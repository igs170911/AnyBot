use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use serde_json::{Value, json};
use test_harness::FakeClock;

use crate::wire::{Wire, WireFailure, WireFuture, WireReply, WireRequest};

pub(crate) type Scripted = Result<WireReply, WireFailure>;

pub(crate) struct FakeWire {
    clock: Arc<FakeClock>,
    calls: Mutex<Vec<u64>>,
    script: Mutex<VecDeque<Scripted>>,
    fail_forever: bool,
}

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl FakeWire {
    pub(crate) fn new(clock: &Arc<FakeClock>, script: Vec<Scripted>) -> Arc<FakeWire> {
        Arc::new(FakeWire {
            clock: Arc::clone(clock),
            calls: Mutex::new(Vec::new()),
            script: Mutex::new(script.into()),
            fail_forever: false,
        })
    }

    pub(crate) fn always_failing(clock: &Arc<FakeClock>) -> Arc<FakeWire> {
        Arc::new(FakeWire {
            clock: Arc::clone(clock),
            calls: Mutex::new(Vec::new()),
            script: Mutex::new(VecDeque::new()),
            fail_forever: true,
        })
    }

    pub(crate) fn calls(&self) -> Vec<u64> {
        locked(&self.calls).clone()
    }

    pub(crate) fn gaps(&self) -> Vec<u64> {
        let times = self.calls();
        times
            .iter()
            .zip(times.iter().skip(1))
            .map(|(before, after)| after.saturating_sub(*before))
            .collect()
    }
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
        let body: Value = request
            .body
            .as_deref()
            .and_then(|text| serde_json::from_str(text).ok())
            .unwrap_or(Value::Null);
        locked(&self.calls).push(self.clock.now_ms());
        let scripted = locked(&self.script).pop_front();
        let answer = match scripted {
            Some(step) => step,
            None if self.fail_forever => Err(WireFailure::Network("down".to_owned())),
            None => Ok(accepted_for(&body)),
        };
        Box::pin(async move { answer })
    }
}
