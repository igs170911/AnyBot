use aes_gcm as _;
use anybot_foundation as _;
use axum as _;
use base64 as _;
use bytes as _;
use getrandom as _;
use hkdf as _;
use libc as _;
use p256 as _;
use postgresql_embedded as _;
use rcgen as _;
use reqwest as _;
use rustls as _;
use rustls_pki_types as _;
use sad_web_push as _;
use serde as _;
use serde_json as _;
use sha2 as _;
use socket2 as _;
use sqlx as _;
use time as _;
use tokio_rustls as _;
use tokio_stream as _;
use toml as _;
use url as _;

use std::sync::Arc;
use std::time::Duration;

use test_harness::fake_model::model_start;
use test_harness::http::{sse_post, test_post};
use test_harness::model_script::{
    Step, model_calls_tool, model_replies_text, model_think_then_reply,
};
use test_harness::realtime::timeout_real;

fn at<'a>(value: &'a serde_json::Value, key: &str) -> &'a serde_json::Value {
    value.get(key).unwrap_or(&serde_json::Value::Null)
}

fn at_index(value: &serde_json::Value, idx: usize) -> &serde_json::Value {
    value.get(idx).unwrap_or(&serde_json::Value::Null)
}

fn anthropic_body(stream: bool) -> String {
    serde_json::json!({ "model": "claude-test", "stream": stream, "messages": [] }).to_string()
}

async fn model_with(steps: Vec<Step>) -> Result<(Arc<test_harness::FakeModel>, u16), String> {
    let (model, port) = timeout_real(model_start(), Duration::from_secs(20)).await??;
    model.script_set(steps);
    Ok((model, port))
}

#[tokio::test]
async fn fake_model_anthropic_stream_text_events_in_order() {
    let (_model, port) = model_with(vec![model_replies_text("hi there", true)])
        .await
        .unwrap();
    let lines = sse_post(
        &format!("http://127.0.0.1:{port}/v1/messages"),
        &anthropic_body(true),
    )
    .await
    .unwrap();
    let joined = lines.join("\n");
    let start = joined.find("message_start").unwrap();
    let delta = joined.find("content_block_delta").unwrap();
    let stop = joined.find("message_stop").unwrap();
    assert!(start < delta && delta < stop);
    assert!(joined.contains("hi there"));
}

#[tokio::test]
async fn fake_model_anthropic_json_blocks_and_tool_use() {
    let args = serde_json::json!({ "city": "taipei" });
    let (_model, port) = model_with(vec![model_calls_tool("weather", args.clone(), false)])
        .await
        .unwrap();
    let (code, body) = test_post(port, "/v1/messages", &anthropic_body(false))
        .await
        .unwrap();
    assert_eq!(code, 200);
    let value: serde_json::Value = serde_json::from_str(&body).unwrap();
    let block = at_index(at(&value, "content"), 0);
    assert_eq!(at(block, "type").as_str().unwrap(), "tool_use");
    assert_eq!(at(block, "name").as_str().unwrap(), "weather");
    assert_eq!(at(block, "input"), &args);
}

#[tokio::test]
async fn fake_model_anthropic_thinking_block_before_text() {
    let (_model, port) = model_with(vec![model_think_then_reply("deep thought", "final", true)])
        .await
        .unwrap();
    let lines = sse_post(
        &format!("http://127.0.0.1:{port}/v1/messages"),
        &anthropic_body(true),
    )
    .await
    .unwrap();
    let joined = lines.join("\n");
    let think = joined.find("thinking_delta").unwrap();
    let text = joined.find("text_delta").unwrap();
    assert!(think < text);
    assert!(joined.contains("deep thought"));
}
