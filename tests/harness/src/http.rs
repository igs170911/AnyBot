use std::io::Read;
use std::io::Write;
use std::net::TcpStream;
use std::time::Duration;

pub fn http_get_addr(addr: &str, path: &str) -> Result<(u16, String), String> {
    let (host, port) = split_addr(addr)?;
    let connect_host = strip_brackets(&host);
    let mut stream =
        TcpStream::connect((connect_host, port)).map_err(|e| format!("連 {addr} 失敗：{e}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(|e| e.to_string())?;
    let request = format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    stream
        .write_all(request.as_bytes())
        .map_err(|e| format!("送請求失敗：{e}"))?;
    let mut raw = Vec::new();
    stream
        .read_to_end(&mut raw)
        .map_err(|e| format!("讀回應失敗：{e}"))?;
    split_response(String::from_utf8_lossy(&raw).into_owned())
}

pub fn http_get(port: u16, path: &str) -> Result<(u16, String), String> {
    http_get_addr(&format!("127.0.0.1:{port}"), path)
}

pub fn http_wait(port: u16, path: &str) -> Result<(u16, String), String> {
    let deadline = std::time::Instant::now().checked_add(Duration::from_secs(10));
    loop {
        let attempt = http_get(port, path);
        if let Ok(result) = attempt {
            return Ok(result);
        }
        if deadline.is_none_or(|d| std::time::Instant::now() >= d) {
            return attempt;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn split_addr(addr: &str) -> Result<(String, u16), String> {
    let (host, tail) = addr.rsplit_once(':').ok_or(format!("位址 {addr} 缺少埠"))?;
    let port = tail
        .parse::<u16>()
        .map_err(|e| format!("位址 {addr} 的埠格式錯：{e}"))?;
    Ok((host.to_owned(), port))
}

fn strip_brackets(host: &str) -> &str {
    if host.starts_with('[') && host.ends_with(']') {
        let end = host.len().saturating_sub(1);
        host.get(1..end).unwrap_or(host)
    } else {
        host
    }
}

fn split_response(text: String) -> Result<(u16, String), String> {
    let first_line = text.lines().next().unwrap_or("");
    let code = first_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("")
        .parse::<u16>()
        .map_err(|e| format!("回應狀態碼格式錯：{e}"))?;
    Ok((code, text))
}
