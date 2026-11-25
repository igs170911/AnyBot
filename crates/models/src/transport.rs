use std::sync::Arc;
use std::time::Duration;

use anybot_foundation::clock::{Clock, ClockExpired, TimestampMs};
use bytes::Bytes;
use futures_util::StreamExt;
use tokio::sync::mpsc;

use crate::where_run::{CONNECT_TIMEOUT, STALL_AFTER_TEXT};

pub(crate) struct HttpBytes {
    pub status: u16,
    pub body: Bytes,
}

#[derive(Debug)]
pub struct PartialBody {
    pub status: u16,
    pub bytes: Vec<u8>,
    pub stalled: bool,
}

pub(crate) struct TimedTransport {
    pub client: reqwest::Client,
    clock: Arc<dyn Clock>,
}

impl TimedTransport {
    pub(crate) fn new(client: reqwest::Client, clock: Arc<dyn Clock>) -> Self {
        Self { client, clock }
    }

    pub(crate) async fn send(
        &self,
        builder: reqwest::RequestBuilder,
        first_wait: Duration,
    ) -> Result<HttpBytes, TransportFail> {
        let started = self.clock.now_ms();
        let response = self.connect(builder, started).await?;
        self.read_body(response, started, first_wait).await
    }

    async fn connect(
        &self,
        builder: reqwest::RequestBuilder,
        _started: TimestampMs,
    ) -> Result<reqwest::Response, TransportFail> {
        let send = builder.send();
        let wait = self.clock.sleep(CONNECT_TIMEOUT);
        tokio::pin!(send);
        tokio::select! {
            biased;
            outcome = &mut send => outcome.map_err(|_| TransportFail::Unreachable),
            () = wait => Err(TransportFail::ConnectTimeout),
        }
    }

    async fn read_body(
        &self,
        response: reqwest::Response,
        started: TimestampMs,
        first_wait: Duration,
    ) -> Result<HttpBytes, TransportFail> {
        let status = response.status().as_u16();
        let mut stream = response.bytes_stream();
        let mut body = Vec::new();
        let mut saw_byte = false;
        loop {
            let wait = if saw_byte {
                STALL_AFTER_TEXT
            } else {
                first_wait
            };
            let next = stream.next();
            let timer = self.clock.sleep(wait);
            tokio::pin!(next);
            tokio::select! {
                biased;
                item = &mut next => {
                    match item {
                        Some(Ok(chunk)) => {
                            saw_byte = true;
                            body.extend_from_slice(&chunk);
                        }
                        Some(Err(_)) if saw_byte => {
                            return Err(TransportFail::Partial(PartialBody {
                                status,
                                bytes: body,
                                stalled: false,
                            }));
                        }
                        Some(Err(_)) => return Err(TransportFail::Dropped),
                        None => break,
                    }
                }
                () = timer => {
                    if saw_byte {
                        return Err(TransportFail::Partial(PartialBody {
                            status,
                            bytes: body,
                            stalled: true,
                        }));
                    }
                    let _mark = started.millis();
                    return Err(TransportFail::FirstToken);
                }
            }
        }
        Ok(HttpBytes {
            status,
            body: Bytes::from(body),
        })
    }

    pub(crate) async fn open_stream(
        &self,
        builder: reqwest::RequestBuilder,
        first_wait: Duration,
    ) -> Result<StreamOpen, TransportFail> {
        let started = self.clock.now_ms();
        let response = self.connect(builder, started).await?;
        let status = response.status().as_u16();
        let (tx, rx) = mpsc::channel(32);
        let clock = Arc::clone(&self.clock);
        tokio::spawn(async move {
            pump_stream(response, tx, clock, first_wait).await;
        });
        Ok(StreamOpen { status, rx })
    }
}

pub(crate) struct StreamOpen {
    pub status: u16,
    pub rx: mpsc::Receiver<Result<Bytes, TransportFail>>,
}

async fn pump_stream(
    response: reqwest::Response,
    tx: mpsc::Sender<Result<Bytes, TransportFail>>,
    clock: Arc<dyn Clock>,
    first_wait: Duration,
) {
    let mut stream = response.bytes_stream();
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
