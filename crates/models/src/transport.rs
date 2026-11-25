use std::sync::Arc;
use std::time::Duration;

use anybot_foundation::clock::{Clock, ClockExpired, TimestampMs};
use bytes::Bytes;
use futures_util::StreamExt;
use tokio::sync::mpsc;

use crate::http_wire::{SharedWire, WireBody, WireRequest, WireResponse};
use crate::where_run::{CONNECT_TIMEOUT, STALL_AFTER_TEXT};

pub(crate) struct HttpBytes {
    pub status: u16,
    pub body: Bytes,
    pub retry_after: Option<u64>,
}

#[derive(Debug)]
pub struct PartialBody {
    pub status: u16,
    pub bytes: Vec<u8>,
    pub stalled: bool,
}

pub(crate) struct TimedTransport {
    wire: SharedWire,
    clock: Arc<dyn Clock>,
}

impl TimedTransport {
    pub(crate) fn clock(&self) -> Arc<dyn Clock> {
        Arc::clone(&self.clock)
    }

    pub(crate) fn new(wire: SharedWire, clock: Arc<dyn Clock>) -> Self {
        Self { wire, clock }
    }

    pub(crate) fn wire(&self) -> SharedWire {
        Arc::clone(&self.wire)
    }

    pub(crate) async fn exchange(
        &self,
        request: &WireRequest,
        first_wait: Duration,
    ) -> Result<HttpBytes, TransportFail> {
        let started = self.clock.now_ms();
        let response = self.connect(request, started).await?;
        let _: Duration = first_wait;
        Ok(HttpBytes {
            status: response.status,
            body: response.body,
            retry_after: retry_after_of(&response.headers),
        })
    }

    async fn connect(
        &self,
        request: &WireRequest,
        _started: TimestampMs,
    ) -> Result<WireResponse, TransportFail> {
        let send = self.wire.send(request);
        let wait = self.clock.sleep(CONNECT_TIMEOUT);
        tokio::pin!(send);
        tokio::select! {
            biased;
            outcome = &mut send => outcome,
            () = wait => Err(TransportFail::ConnectTimeout),
        }
    }

    pub(crate) async fn open_stream(
        &self,
        request: &WireRequest,
        first_wait: Duration,
    ) -> Result<StreamOpen, TransportFail> {
        let started = self.clock.now_ms();
        let open = self.open_wire(request, started).await?;
        let status = open.status;
        let headers = open.headers;
        let (tx, rx) = mpsc::channel(32);
        let clock = Arc::clone(&self.clock);
        tokio::spawn(async move {
            pump_body(open.body, tx, clock, first_wait).await;
        });
        Ok(StreamOpen {
            status,
            headers,
            rx,
        })
    }

    async fn open_wire(
        &self,
        request: &WireRequest,
        _started: TimestampMs,
    ) -> Result<crate::http_wire::WireOpen, TransportFail> {
        let send = self.wire.open(request);
        let wait = self.clock.sleep(CONNECT_TIMEOUT);
        tokio::pin!(send);
        tokio::select! {
            biased;
            outcome = &mut send => outcome,
            () = wait => Err(TransportFail::ConnectTimeout),
        }
    }
}

pub(crate) struct StreamOpen {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub rx: mpsc::Receiver<Result<Bytes, TransportFail>>,
}

pub(crate) fn retry_after_of(headers: &[(String, String)]) -> Option<u64> {
    headers.iter().find_map(|(name, value)| {
        if name.eq_ignore_ascii_case("retry-after") {
            value.parse().ok()
        } else {
            None
        }
    })
}

async fn pump_body(
    body: WireBody,
    tx: mpsc::Sender<Result<Bytes, TransportFail>>,
    clock: Arc<dyn Clock>,
    first_wait: Duration,
) {
    match body {
        WireBody::Stream(stream) => pump_stream(stream, tx, clock, first_wait).await,
    }
}

async fn pump_stream(
    mut stream: std::pin::Pin<
        Box<dyn futures_util::Stream<Item = Result<Bytes, TransportFail>> + Send>,
    >,
    tx: mpsc::Sender<Result<Bytes, TransportFail>>,
    clock: Arc<dyn Clock>,
    first_wait: Duration,
) {
    let mut saw_byte = false;
    let mut held = Vec::new();
    loop {
        let wait = if saw_byte {
            STALL_AFTER_TEXT
        } else {
            first_wait
        };
        let next = stream.next();
        let timer = clock.sleep(wait);
        tokio::pin!(next);
        let item = tokio::select! {
            biased;
            item = &mut next => item,
            () = timer => {
                let fail = if saw_byte {
                    TransportFail::Partial(PartialBody {
                        status: 200,
                        bytes: held,
                        stalled: true,
                    })
                } else {
                    TransportFail::FirstToken
                };
                let _sent = tx.send(Err(fail)).await.is_ok();
                return;
            }
        };
        match item {
            Some(Ok(chunk)) => {
                saw_byte = true;
                held.extend_from_slice(&chunk);
                if tx.send(Ok(chunk)).await.is_err() {
                    return;
                }
            }
            Some(Err(_)) => {
                let fail = if saw_byte {
                    TransportFail::Partial(PartialBody {
                        status: 200,
                        bytes: held,
                        stalled: false,
                    })
                } else {
                    TransportFail::Dropped
                };
                let _sent = tx.send(Err(fail)).await.is_ok();
                return;
            }
            None => return,
        }
    }
}

#[derive(Debug)]
pub enum TransportFail {
    ConnectTimeout,
    Unreachable,
    Certificate(anybot_foundation::CertFault),
    FirstToken,
    Stalled { partial: Vec<u8> },
    Dropped,
    Partial(PartialBody),
}

impl From<ClockExpired> for TransportFail {
    fn from(_expired: ClockExpired) -> Self {
        Self::ConnectTimeout
    }
}
