use std::time::Duration;

use reqwest::Client;
use reqwest::Method;
use reqwest::header::{ACCEPT, AUTHORIZATION, HeaderValue};
use tokio::sync::mpsc;

pub const SSE_DEADLINE: Duration = Duration::from_secs(25);

#[derive(Debug)]
pub struct SseOpen {
    rx: mpsc::Receiver<String>,
    pub headers: reqwest::header::HeaderMap,
}

impl SseOpen {
    pub async fn recv_block(&mut self) -> Result<String, String> {
        let got = tokio::time::timeout(SSE_DEADLINE, self.rx.recv())
            .await
            .map_err(|_| "no sse block inside the deadline".to_owned())?;
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

    pub fn close(self) {
        drop(self.rx);
    }
}

pub async fn open_sse(
    base: &str,
    path: &str,
    bearer: &str,
    query: &str,
) -> Result<SseOpen, String> {
    open_sse_with(base, path, bearer, query, None, None, None).await
}

fn apply_sec_fetch(
    request: reqwest::RequestBuilder,
    sec_fetch: Option<&str>,
) -> Result<reqwest::RequestBuilder, String> {
    Ok(match sec_fetch {
        Some(value) => request.header(
            reqwest::header::HeaderName::from_static("sec-fetch-site"),
            HeaderValue::from_str(value).map_err(|e| e.to_string())?,
        ),
        None => request,
    })
}

fn apply_cookie_and_origin(
    request: reqwest::RequestBuilder,
    cookie: Option<&str>,
    origin: Option<&str>,
) -> Result<reqwest::RequestBuilder, String> {
    let request = match cookie {
        Some(value) => request.header(
            reqwest::header::COOKIE,
            HeaderValue::from_str(value).map_err(|e| e.to_string())?,
        ),
        None => request,
    };
    Ok(match origin {
        Some(value) => request.header(
            reqwest::header::ORIGIN,
            HeaderValue::from_str(value).map_err(|e| e.to_string())?,
        ),
        None => request,
    })
}

pub async fn open_sse_with(
    base: &str,
    path: &str,
    bearer: &str,
    query: &str,
    cookie: Option<&str>,
    origin: Option<&str>,
    sec_fetch: Option<&str>,
) -> Result<SseOpen, String> {
    let url = format!("{base}{path}?{query}");
    let client = Client::builder()
        .read_timeout(SSE_DEADLINE.saturating_mul(2))
        .build()
        .map_err(|e| e.to_string())?;
    let request = client
        .request(Method::GET, &url)
        .header(ACCEPT, HeaderValue::from_static("text/event-stream"));
    let request = if bearer.is_empty() {
        request
    } else {
        request.header(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {bearer}")).map_err(|e| e.to_string())?,
        )
    };
    let request = apply_sec_fetch(request, sec_fetch)?;
    let request = apply_cookie_and_origin(request, cookie, origin)?;
    let request = request.send().await.map_err(|e| e.to_string())?;
    let status = request.status().as_u16();
    if status != 200 {
        let body = request.text().await.map_err(|e| e.to_string())?;
        return Err(format!("sse open failed with {status}: {body}"));
    }
    let headers = request.headers().clone();
    let (tx, rx) = mpsc::channel(32);
    tokio::spawn(pump(request, tx));
    Ok(SseOpen { rx, headers })
}

async fn pump(request: reqwest::Response, tx: mpsc::Sender<String>) {
    let mut buffered: Vec<u8> = Vec::new();
    let mut stream = request;
    loop {
        let Ok(Some(bytes)) = stream.chunk().await else {
            break;
        };
        buffered.extend_from_slice(&bytes);
        while let Some(at) = find_double_newline(&buffered) {
            let Some(cut) = at.checked_add(2) else {
                return;
            };
            let part: Vec<u8> = buffered.drain(..cut).collect();
            let block = String::from_utf8(part)
                .map_err(|e| e.to_string())
                .unwrap_or_default();
            if tx.send(block).await.is_err() {
                return;
            }
        }
    }
}

fn find_double_newline(bytes: &[u8]) -> Option<usize> {
    bytes.windows(2).position(|w| w == b"\n\n")
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
