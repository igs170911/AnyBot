use std::time::Duration;

use reqwest::Method;
use tokio_stream::StreamExt;

pub async fn http_get_addr(addr: &str, path: &str) -> Result<(u16, String), String> {
    let client = plain_client()?;
    request(&client, Method::GET, &format!("http://{addr}{path}"), None).await
}

pub async fn http_get(port: u16, path: &str) -> Result<(u16, String), String> {
    http_get_addr(&format!("127.0.0.1:{port}"), path).await
}

pub async fn test_get_async(port: u16, path: &str) -> Result<(u16, String), String> {
    http_get(port, path).await
}

pub async fn test_post(port: u16, path: &str, body: &str) -> Result<(u16, String), String> {
    let client = plain_client()?;
    request(
        &client,
        Method::POST,
        &format!("http://127.0.0.1:{port}{path}"),
        Some(body),
    )
    .await
}

pub async fn test_put(port: u16, path: &str, body: &str) -> Result<(u16, String), String> {
    let client = plain_client()?;
    request(
        &client,
        Method::PUT,
        &format!("http://127.0.0.1:{port}{path}"),
        Some(body),
    )
    .await
}

pub async fn test_delete(port: u16, path: &str) -> Result<(u16, String), String> {
    let client = plain_client()?;
    request(
        &client,
        Method::DELETE,
        &format!("http://127.0.0.1:{port}{path}"),
        None,
    )
    .await
}

pub async fn request(
    client: &reqwest::Client,
    method: Method,
    url: &str,
    body: Option<&str>,
) -> Result<(u16, String), String> {
    let mut builder = client.request(method.clone(), url);
    if let Some(text) = body {
        builder = builder.body(text.to_owned());
    }
    let response = builder
        .send()
        .await
        .map_err(|e| format!("{method} {url} failed: {e}"))?;
    let code = response.status().as_u16();
    let text = response
        .text()
        .await
        .map_err(|e| format!("read response from {url} failed: {e}"))?;
    Ok((code, text))
}

pub fn plain_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("build HTTP client failed: {e}"))
}

pub fn client_via_proxy(proxy_addr: &str) -> Result<reqwest::Client, String> {
    let proxy = reqwest::Proxy::all(proxy_addr)
        .map_err(|e| format!("build proxy {proxy_addr} failed: {e}"))?;
    reqwest::Client::builder()
        .proxy(proxy)
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("build proxied client failed: {e}"))
}

pub async fn sse_lines(url: &str) -> Result<Vec<String>, String> {
    let client = plain_client()?;
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("GET {url} failed: {e}"))?;
    let code = response.status().as_u16();
    if code != 200 {
        return Err(format!("SSE {url} returned status {code}"));
    }
    collect_sse_lines(response).await
}

async fn collect_sse_lines(response: reqwest::Response) -> Result<Vec<String>, String> {
    let mut lines = Vec::new();
    let deadline = Duration::from_secs(20);
    let streamed = response.bytes_stream();
    tokio::pin!(streamed);
    loop {
        let next = tokio::time::timeout(deadline, streamed.next()).await;
        let Ok(Some(Ok(bytes))) = next else {
            break;
        };
        for line in String::from_utf8_lossy(&bytes).lines() {
            if !line.is_empty() {
                lines.push(line.to_owned());
            }
        }
        if lines.iter().any(|l| l.trim() == "[DONE]") {
            break;
        }
    }
    Ok(lines)
}

pub async fn sse_post(url: &str, body: &str) -> Result<Vec<String>, String> {
    let client = plain_client()?;
    let response = client
        .post(url)
        .body(body.to_owned())
        .send()
        .await
        .map_err(|e| format!("POST {url} failed: {e}"))?;
    let code = response.status().as_u16();
    if code != 200 {
        return Err(format!("SSE POST {url} returned status {code}"));
    }
    collect_sse_lines(response).await
}

pub async fn sse_wait_for_line(
    url: &str,
    needle: &str,
    span: Duration,
) -> Result<Vec<String>, String> {
    let client = plain_client()?;
    let response = client
        .post(url)
        .body(
            serde_json::json!({ "model": "claude-test", "stream": true, "messages": [] })
                .to_string(),
        )
        .send()
        .await
        .map_err(|e| format!("GET {url} failed: {e}"))?;
    let mut lines = Vec::new();
    let streamed = response.bytes_stream();
    tokio::pin!(streamed);
    let overall = tokio::time::timeout(span, async move {
        loop {
            let next = streamed.next().await;
            let Some(Ok(bytes)) = next else {
                return Ok(lines);
            };
            for line in String::from_utf8_lossy(&bytes).lines() {
                if !line.is_empty() {
                    lines.push(line.to_owned());
                }
            }
            if lines.iter().any(|l| l.contains(needle)) {
                return Ok(lines);
            }
        }
    });
    overall
        .await
        .map_err(|_e| format!("SSE {url} did not produce a line containing {needle}"))?
}

pub async fn http_wait(port: u16, path: &str) -> Result<(u16, String), String> {
    let deadline = std::time::Instant::now().checked_add(Duration::from_secs(10));
    loop {
        let attempt = http_get(port, path).await;
        if let Ok(result) = attempt {
            return Ok(result);
        }
        if deadline.is_none_or(|d| std::time::Instant::now() >= d) {
            return attempt;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
