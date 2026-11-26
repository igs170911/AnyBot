use serde_json::{Value, json};

use anybot_app_server as _;
use anybot_app_test as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_messages as _;
use anybot_server as _;
use anybot_server_test as _;
use anybot_store_server as _;
use axum as _;
use clap as _;
use rpassword as _;
use serde_json as _;
use sqlx as _;
use test_harness as _;
use tokio as _;
use tracing as _;

use anybot_app_test::http::{RequestSpec, get, post};
use anybot_app_test::serve::serve_start;
use anybot_app_test::sse::{data_json, event_name, open_sse, open_sse_with};
use anybot_server_test::cookie_of;
use anybot_server_test::{make_admin, table_counts};

async fn cookie_login(
    served: &anybot_app_test::serve::Served,
    key: &str,
) -> Result<String, String> {
    let answer = anybot_server_test::post_cookie_login(
        served,
        "alice",
        key,
        Some(&served.base_url()),
        &served.base_url(),
    )
    .await?;
    cookie_of(&answer)
}

async fn sse_seed(base: &str, cookie: &str, origin: &str, key: &str) -> Result<(), String> {
    let body = json!({
        "stream": "account",
        "items": [{
            "device_id": "dev-4",
            "kind": "message",
            "content": {"text": "dolphin"},
            "created_at_ms": 1_700_000_000_100_i64,
            "offline_sent": false,
        }],
    })
    .to_string();
    let spec = RequestSpec::new(&body)
        .with_key(key)
        .with_header("Cookie", cookie)
        .with_header("anybot-csrf", "1")
        .with_header("Origin", origin);
    let answer = post(base, "/v1/streams/append", &spec).await?;
    assert_eq!(answer.status, 200, "seed: {}", answer.json);
    Ok(())
}

#[tokio::test]
async fn t_conn_auth_4() -> Result<(), String> {
    let served = serve_start("t_conn_auth_4").await?;
    let base = served.base_url();
    anybot_app_test::serve::boot_pair(&served).await?;
    let cookie = cookie_login(&served, "k-a4-login").await?;
    sse_seed(&base, &cookie, &base, "k-a4-seed").await?;
    let foreign = open_sse_with(
        &base,
        "/v1/streams/notifications",
        "",
        "",
        Some(&cookie),
        Some("https://other.example"),
        Some("cross-site"),
    )
    .await;
    match foreign {
        Ok(opened) => {
            opened.close();
            return Err("cookie sse with a foreign origin must be refused".to_owned());
        }
        Err(refused) => {
            assert!(refused.contains("403"), "foreign cookie sse: {refused}");
        }
    }
    let mut good = open_sse_with(
        &base,
        "/v1/streams/notifications",
        "",
        "",
        Some(&cookie),
        Some(&base),
        Some("same-origin"),
    )
    .await?;
    let block = good.recv_block().await?;
    assert_eq!(
        event_name(&block).as_deref(),
        Some("progress"),
        "head: {block}"
    );
    let json = data_json(&block)?;
    let seq = json
        .get("streams")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .and_then(|row| row.get("seq"))
        .and_then(Value::as_i64);
    assert_eq!(seq, Some(1), "cookie sse head: {json}");
    good.close();
    served.stop().await?;
    Ok(())
}

async fn sse_heads_stable(base: &str, admin: &str) -> Result<(), String> {
    let mut stream_a = open_sse(base, "/v1/streams/notifications", admin, "").await?;
    let block_a = stream_a.recv_block().await?;
    let head_a = data_json(&block_a).map_err(|e| format!("head a: {block_a} | {e}"))?;
    stream_a.close();
    let mut stream_b = open_sse(base, "/v1/streams/notifications", admin, "").await?;
    let block_b = stream_b.recv_block().await?;
    let head_b = data_json(&block_b).map_err(|e| format!("head b: {block_b} | {e}"))?;
    stream_b.close();
    assert_eq!(head_a, head_b, "sse heads must be stable");
    Ok(())
}

#[tokio::test]
async fn t_conn_auth_6() -> Result<(), String> {
    let served = serve_start("t_conn_auth_6").await?;
    let base = served.base_url();
    let admin = make_admin(&served).await?;
    let seed = json!({
        "stream": "account",
        "items": [{
            "device_id": "dev-6",
            "kind": "message",
            "content": {"text": "stable"},
            "created_at_ms": 1_700_000_000_100_i64,
            "offline_sent": false,
        }],
    })
    .to_string();
    let seeded = post(
        &base,
        "/v1/streams/append",
        &RequestSpec::new(&seed)
            .with_key("k-a6-seed")
            .with_bearer(&admin),
    )
    .await?;
    assert_eq!(seeded.status, 200, "seed: {}", seeded.json);
    let after_spec = || RequestSpec::new("").with_bearer(&admin).without_version();
    let first = get(
        &base,
        "/v1/streams/after?stream=account%2Falice&after=0",
        &after_spec(),
    )
    .await?;
    let second = get(
        &base,
        "/v1/streams/after?stream=account%2Falice&after=0",
        &after_spec(),
    )
    .await?;
    assert_eq!(first.status, 200, "after first: {}", first.json);
    assert_eq!(first.json, second.json, "after must be stable");
    sse_heads_stable(&base, &admin).await?;
    let before = table_counts(&served).await?;
    for path in ["/v1/health", "/v1/server/status", "/v1/admin/invite"] {
        let spec = if path == "/v1/admin/invite" {
            RequestSpec::new("{}").with_bearer(&admin)
        } else {
            RequestSpec::new("{}")
        };
        let answer = get(&base, path, &spec).await?;
        assert_eq!(answer.status, 200, "{path}: {}", answer.json);
    }
    let after = table_counts(&served).await?;
    assert_eq!(before, after, "GET routes must not change stored data");
    served.stop().await?;
    Ok(())
}
