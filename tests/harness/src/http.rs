use std::time::Duration;

pub async fn http_get_addr(addr: &str, path: &str) -> Result<(u16, String), String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("建 HTTP 客戶端失敗：{e}"))?;
    let url = format!("http://{addr}{path}");
    let response = client
        .get(url.clone())
        .send()
        .await
        .map_err(|e| format!("連 {url} 失敗：{e}"))?;
    let code = response.status().as_u16();
    let body = response
        .text()
        .await
        .map_err(|e| format!("讀 {url} 回應失敗：{e}"))?;
    Ok((code, body))
}

pub async fn http_get(port: u16, path: &str) -> Result<(u16, String), String> {
    http_get_addr(&format!("127.0.0.1:{port}"), path).await
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
