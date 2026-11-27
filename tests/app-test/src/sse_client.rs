use reqwest::Client;
use reqwest::Method;
use reqwest::header::{ACCEPT, AUTHORIZATION, HeaderName, HeaderValue};
use tokio::sync::mpsc;

use crate::sse::{SSE_CONNECT_RULE, SSE_DEADLINE, SseOpen};

fn apply_sec_fetch(
    request: reqwest::RequestBuilder,
    sec_fetch: Option<&str>,
) -> Result<reqwest::RequestBuilder, String> {
    Ok(match sec_fetch {
        Some(value) => request.header(
            HeaderName::from_static("sec-fetch-site"),
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

pub(crate) fn open_client(port: Option<u16>) -> Result<Client, String> {
    let mut builder = Client::builder()
        .read_timeout(SSE_DEADLINE.saturating_mul(2))
        .timeout(SSE_CONNECT_RULE)
        .connect_timeout(SSE_CONNECT_RULE);
    if let Some(proxy_port) = port {
        let proxy = reqwest::Proxy::all(format!("http://127.0.0.1:{proxy_port}"))
            .map_err(|e| format!("build proxy failed: {e}"))?;
        builder = builder.proxy(proxy);
    }
    builder.build().map_err(|e| e.to_string())
}

pub(crate) async fn send_request(
    client: &Client,
    url: &str,
    bearer: &str,
    cookie: Option<&str>,
    origin: Option<&str>,
    sec_fetch: Option<&str>,
    version: Option<&str>,
) -> Result<reqwest::Response, String> {
    let request = client
        .request(Method::GET, url)
        .header(ACCEPT, HeaderValue::from_static("text/event-stream"));
    let request = match version {
        Some(seen) => request.header(
            HeaderName::from_static("anybot-client-version"),
            HeaderValue::from_str(seen).map_err(|e| e.to_string())?,
        ),
        None => request,
    };
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
    Ok(request)
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
    bytes.windows(2).position(|w| {
        w == b"

"
    })
}

pub(crate) async fn receive(request: reqwest::Response) -> Result<SseOpen, String> {
    let status = request.status().as_u16();
    if status != 200 {
        let body = request.text().await.map_err(|e| e.to_string())?;
        return Err(format!("sse open failed with {status}: {body}"));
    }
    let headers = request.headers().clone();
    let (tx, rx) = mpsc::channel(32);
    tokio::spawn(pump(request, tx));
    Ok(SseOpen::from_parts(rx, headers))
}
