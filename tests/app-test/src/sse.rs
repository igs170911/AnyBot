use std::time::Duration;

use crate::sse_client::{open_client, receive, send_request};
use tokio::sync::mpsc;

pub const SSE_DEADLINE: Duration = Duration::from_secs(25);
pub const SSE_IDLE_RULE: Duration = Duration::from_secs(30);
pub const SSE_CONNECT_RULE: Duration = Duration::from_secs(30);

#[derive(Debug)]
pub struct SseOpen {
    rx: mpsc::Receiver<String>,
    pub headers: reqwest::header::HeaderMap,
}

impl SseOpen {
    #[must_use]
    pub(crate) fn from_parts(
        rx: mpsc::Receiver<String>,
        headers: reqwest::header::HeaderMap,
    ) -> Self {
        SseOpen { rx, headers }
    }

    #[must_use]
    pub fn silent(rx: mpsc::Receiver<String>) -> Self {
        SseOpen {
            rx,
            headers: reqwest::header::HeaderMap::new(),
        }
    }

    pub async fn recv_block(&mut self) -> Result<String, String> {
        let got = tokio::time::timeout(SSE_DEADLINE, self.rx.recv())
            .await
            .map_err(|_| "no sse block inside the deadline".to_owned())?;
        got.ok_or("sse stream ended".to_owned())
    }

    pub async fn recv_within(&mut self, idle: Duration) -> Result<String, String> {
        let got = tokio::time::timeout(idle, self.rx.recv())
            .await
            .map_err(|_| "no sse block inside the idle window".to_owned())?;
        got.ok_or("sse stream ended".to_owned())
    }

    pub async fn recv_json_of(&mut self, name: &str) -> Result<serde_json::Value, String> {
        for _round in 0..20 {
            let block = self.recv_block().await?;
            if event_name(&block).as_deref() == Some(name) {
                return data_json(&block);
            }
        }
        Err(format!("no event named {name} on the stream"))
    }

    pub async fn count_heartbeats(&mut self, want: usize) -> Result<usize, String> {
        let mut seen = 0usize;
        for _round in 0..40 {
            let block = self.recv_block().await?;
            if block.starts_with(':') {
                let Some(next) = seen.checked_add(1) else {
                    return Err("heartbeat counter overflow".to_owned());
                };
                seen = next;
                if seen >= want {
                    return Ok(seen);
                }
            }
        }
        Err(format!("only {seen} heartbeats seen"))
    }

    pub async fn count_heartbeats_within(&mut self, window: Duration) -> Result<usize, String> {
        let mut seen = 0usize;
        let mut left = window;
        while !left.is_zero() {
            let started = std::time::Instant::now();
            let Ok(Some(block)) = tokio::time::timeout(left, self.rx.recv()).await else {
                return Ok(seen);
            };
            let spent = started.elapsed();
            let taken = Duration::from_millis(u64::try_from(spent.as_millis()).unwrap_or(0));
            left = left.saturating_sub(taken);
            if block.starts_with(':') {
                seen = seen
                    .checked_add(1)
                    .ok_or("heartbeat counter overflow".to_owned())?;
            }
        }
        Ok(seen)
    }

    pub fn drain_now(&mut self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        while let Ok(block) = self.rx.try_recv() {
            out.push(block);
        }
        out
    }

    pub async fn recv_data_block(&mut self) -> Result<String, String> {
        for _round in 0..20 {
            let block = self.recv_block().await?;
            if !block.starts_with(':') {
                return Ok(block);
            }
        }
        Err("no data block between heartbeats".to_owned())
    }

    pub async fn recv_ended(&mut self, within: Duration) -> Result<(), String> {
        let got = tokio::time::timeout(within, self.rx.recv())
            .await
            .map_err(|_| "the sse stream stayed open past the deadline".to_owned())?;
        if got.is_some() {
            let rest = within.checked_sub(SSE_DEADLINE).unwrap_or(Duration::ZERO);
            return Box::pin(self.recv_ended(rest)).await;
        }
        Ok(())
    }

    pub fn close(&mut self) {
        let (sender, dummy) = mpsc::channel(1);
        let _kept = sender;
        let real = std::mem::replace(&mut self.rx, dummy);
        drop(real);
    }
}

#[derive(Debug, Clone)]
pub struct SseRequest<'a> {
    pub base: &'a str,
    pub path: &'a str,
    pub bearer: &'a str,
    pub query: &'a str,
    pub cookie: Option<&'a str>,
    pub origin: Option<&'a str>,
    pub sec_fetch: Option<&'a str>,
    pub version: Option<&'a str>,
}

impl<'a> SseRequest<'a> {
    #[must_use]
    pub fn new(base: &'a str, path: &'a str, bearer: &'a str, query: &'a str) -> Self {
        SseRequest {
            base,
            path,
            bearer,
            query,
            cookie: None,
            origin: None,
            sec_fetch: None,
            version: Some("9.9.9"),
        }
    }

    #[must_use]
    pub fn with_cookie(mut self, cookie: &'a str) -> Self {
        self.cookie = Some(cookie);
        self
    }

    #[must_use]
    pub fn with_origin(mut self, origin: &'a str, sec_fetch: &'a str) -> Self {
        self.origin = Some(origin);
        self.sec_fetch = Some(sec_fetch);
        self
    }

    #[must_use]
    pub fn with_version(mut self, version: &'a str) -> Self {
        self.version = Some(version);
        self
    }

    #[must_use]
    pub fn without_version(mut self) -> Self {
        self.version = None;
        self
    }
}

pub async fn open_sse(
    base: &str,
    path: &str,
    bearer: &str,
    query: &str,
) -> Result<SseOpen, String> {
    open_sse_with(&SseRequest::new(base, path, bearer, query)).await
}

pub async fn open_sse_with(request: &SseRequest<'_>) -> Result<SseOpen, String> {
    let client = open_client(None)?;
    let response = send_request(
        &client,
        &format!("{}{}?{}", request.base, request.path, request.query),
        request.bearer,
        request.cookie,
        request.origin,
        request.sec_fetch,
        request.version,
    )
    .await?;
    receive(response).await
}

pub async fn open_sse_via(
    origin: std::net::SocketAddr,
    request: &SseRequest<'_>,
    proxy_port: u16,
) -> Result<SseOpen, String> {
    let client = open_client(Some(proxy_port))?;
    let response = send_request(
        &client,
        &format!("http://{origin}{}?{}", request.path, request.query),
        request.bearer,
        request.cookie,
        None,
        Some("same-origin"),
        request.version,
    )
    .await?;
    receive(response).await
}

#[must_use]
pub fn event_name(block: &str) -> Option<String> {
    for line in block.lines() {
        if let Some(name) = line.strip_prefix("event: ") {
            return Some(name.to_owned());
        }
    }
    None
}

pub fn data_json(block: &str) -> Result<serde_json::Value, String> {
    for line in block.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
            return serde_json::from_str(data).map_err(|e| e.to_string());
        }
    }
    Err(format!("no data line in block: {block}"))
}
