use std::sync::Arc;

use anybot_foundation::Clock;
use futures_util::StreamExt as _;

use crate::http_gate::ConnectGate;
use crate::http_wire::SERVER_TIME_HEADER;
use crate::reconnect_status::RequestGuard;
use crate::wire::{WireFailure, WireLimits, WireReply};

pub(crate) async fn send(
    clock: &Arc<dyn Clock>,
    gate: &Arc<ConnectGate>,
    limits: WireLimits,
    builder: reqwest::RequestBuilder,
) -> Result<reqwest::Response, WireFailure> {
    let mut pending = Box::pin(builder.send());
    let early = tokio::select! {
        biased;
        done = &mut pending => Some(done),
        () = gate.wait_connected() => None,
    };
    if let Some(done) = early {
        return done.map_err(|e| failure_of(&e, gate));
    }
    let expired = clock.sleep(limits.wait);
    tokio::select! {
        biased;
        done = pending => done.map_err(|e| failure_of(&e, gate)),
        () = expired => Err(WireFailure::WaitTimeout),
    }
}

fn failure_of(error: &reqwest::Error, gate: &Arc<ConnectGate>) -> WireFailure {
    if gate.timed_out() {
        WireFailure::ConnectTimeout
    } else if error.is_connect() {
        WireFailure::ConnectFailed(error.to_string())
    } else {
        WireFailure::Network(error.to_string())
    }
}

pub(crate) async fn collect(
    clock: &Arc<dyn Clock>,
    limits: WireLimits,
    response: reqwest::Response,
    guard: &RequestGuard,
) -> Result<WireReply, WireFailure> {
    let status = response.status().as_u16();
    let server_time_ms = response
        .headers()
        .get(SERVER_TIME_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(|text| text.parse::<i64>().ok());
    let mut stream = Box::pin(response.bytes_stream());
    let mut body = Vec::new();
    loop {
        let expired = clock.sleep(limits.wait);
        let next = tokio::select! {
            biased;
            chunk = stream.next() => chunk,
            () = expired => return Err(WireFailure::WaitTimeout),
        };
        match next {
            None => break,
            Some(Ok(bytes)) => {
                guard.bytes_arrived();
                body.extend_from_slice(&bytes);
            }
            Some(Err(error)) => return Err(WireFailure::Network(error.to_string())),
        }
    }
    Ok(WireReply {
        status,
        body,
        server_time_ms,
    })
}
