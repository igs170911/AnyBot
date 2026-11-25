use std::net::TcpListener as StdTcpListener;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use crate::{AppState, build_router};

fn start_test_server() -> u16 {
    let std_listener = StdTcpListener::bind("127.0.0.1:0").unwrap();
    let port = std_listener.local_addr().unwrap().port();
    std_listener.set_nonblocking(true).unwrap();
    let listener = TcpListener::from_std(std_listener).unwrap();
    let app = build_router(AppState::new("test-9.9.9"));
    tokio::spawn(async move {
        match axum::serve(listener, app).await {
            Ok(()) => {}
            Err(err) => {
                let mut stderr = std::io::stderr();
                let _result =
                    std::io::Write::write_fmt(&mut stderr, format_args!("測試伺服器停止：{err}\n"));
            }
        }
    });
    port
}

async fn get_response(port: u16, path: &str) -> (String, String) {
    let stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let (mut reader, mut writer) = stream.into_split();
    let request = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    writer.write_all(request.as_bytes()).await.unwrap();
    let mut raw = Vec::new();
    reader.read_to_end(&mut raw).await.unwrap();
    let text = String::from_utf8(raw).unwrap();
    let (head, body) = text.split_once("\r\n\r\n").unwrap();
    (head.to_owned(), body.to_owned())
}

#[tokio::test]
async fn test_health_ok_json_matches_schema_fields() {
    let port = start_test_server();
    let (head, body) = get_response(port, "/v1/health").await;
    assert!(head.contains("HTTP/1.1 200 OK"));
    assert!(head.contains("content-type: application/json"));
    let value: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        value.get("version").and_then(|v| v.as_str()),
        Some("test-9.9.9")
    );
    assert_eq!(
        value
            .get("protocol_version")
            .and_then(serde_json::Value::as_u64),
        Some(1)
    );
    let now = value.get("now").and_then(|v| v.as_str()).unwrap();
    assert!(now.contains('T') && now.ends_with('Z'));
}

#[tokio::test]
async fn test_health_unknown_path_is_not_found() {
    let port = start_test_server();
    let (head, _body) = get_response(port, "/v1/nosuch").await;
    assert!(head.contains("HTTP/1.1 404"));
}
