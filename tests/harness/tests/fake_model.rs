use aes_gcm as _;
use anybot_foundation as _;
use anybot_store_server as _;
use axum as _;
use base64 as _;
use bytes as _;
use getrandom as _;
use hkdf as _;
use libc as _;
use p256 as _;
use postgresql_embedded as _;
use reqwest as _;
use sad_web_push as _;
use serde as _;
use sha2 as _;
use socket2 as _;
use sqlx as _;
use tokio_stream as _;
use toml as _;
use tracing as _;
use tracing_subscriber as _;
use url as _;

use std::sync::Arc;
use std::time::Duration;

use test_harness::fake_model::{ModelFormat, model_start_with_format};
use test_harness::http::{sse_post, sse_wait_for_line, test_post};
use test_harness::model_script::{
    HeaderValueKind, Step, model_calls_tool, model_cut_off, model_no_response, model_replies_text,
    model_reply_status, model_stall,
};
use test_harness::realtime::timeout_real;

const STOP_MS: &str = "[DONE]";

fn at<'a>(value: &'a serde_json::Value, key: &str) -> &'a serde_json::Value {
    value.get(key).unwrap_or(&serde_json::Value::Null)
}

fn at_index(value: &serde_json::Value, idx: usize) -> &serde_json::Value {
    value.get(idx).unwrap_or(&serde_json::Value::Null)
}

fn choice0(value: &serde_json::Value) -> &serde_json::Value {
    at_index(at(value, "choices"), 0)
}

fn openai_body(stream: bool) -> String {
    serde_json::json!({ "model": "gpt-test", "stream": stream, "messages": [] }).to_string()
}

fn anthropic_body(stream: bool) -> String {
    serde_json::json!({ "model": "claude-test", "stream": stream, "messages": [] }).to_string()
}

async fn model_with(steps: Vec<Step>) -> Result<(Arc<test_harness::FakeModel>, u16), String> {
    model_with_format(steps, ModelFormat::OpenAi).await
}

async fn model_with_format(
    steps: Vec<Step>,
    format: ModelFormat,
) -> Result<(Arc<test_harness::FakeModel>, u16), String> {
    let (model, port) =
        timeout_real(model_start_with_format(format), Duration::from_secs(20)).await??;
    model.script_set(steps);
    Ok((model, port))
}

#[tokio::test]
async fn fake_model_openai_stream_text_ends_with_done() {
    let (_model, port) = model_with(vec![model_replies_text("hello world", true)])
        .await
        .unwrap();
    let lines = sse_post(
        &format!("http://127.0.0.1:{port}/v1/chat/completions"),
        &openai_body(true),
    )
    .await
    .unwrap();
    assert!(lines.iter().any(|l| l.contains("hello world")));
    assert!(lines.iter().any(|l| l.contains(STOP_MS)));
    assert!(lines.iter().any(|l| l.contains("finish_reason")));
}

#[tokio::test]
async fn fake_model_openai_json_text_has_content_and_finish_reason() {
    let (_model, port) = model_with(vec![model_replies_text("plain reply", false)])
        .await
        .unwrap();
    let (code, body) = test_post(port, "/v1/chat/completions", &openai_body(false))
        .await
        .unwrap();
    assert_eq!(code, 200);
    let value: serde_json::Value = serde_json::from_str(&body).unwrap();
    let message = at(choice0(&value), "message");
    assert_eq!(at(message, "content").as_str().unwrap(), "plain reply");
    assert_eq!(
        choice0(&value)
            .get("finish_reason")
            .unwrap()
            .as_str()
            .unwrap(),
        "stop"
    );
}

#[tokio::test]
async fn fake_model_openai_tool_call_fields_in_json_and_stream() {
    let args = serde_json::json!({ "city": "taipei" });
    let (_model, port) = model_with(vec![model_calls_tool("weather", args.clone(), false)])
        .await
        .unwrap();
    let (code, body) = test_post(port, "/v1/chat/completions", &openai_body(false))
        .await
        .unwrap();
    assert_eq!(code, 200);
    let value: serde_json::Value = serde_json::from_str(&body).unwrap();
    let call = at_index(at(at(choice0(&value), "message"), "tool_calls"), 0);
    let function = at(call, "function");
    assert_eq!(at(function, "name").as_str().unwrap(), "weather");
    let parsed: serde_json::Value =
        serde_json::from_str(at(function, "arguments").as_str().unwrap()).unwrap();
    assert_eq!(parsed, args);
    assert_eq!(
        choice0(&value)
            .get("finish_reason")
            .unwrap()
            .as_str()
            .unwrap(),
        "tool_calls"
    );
}

#[tokio::test]
async fn fake_model_openai_cut_off_has_head_then_no_done() {
    let (_model, port) = model_with(vec![model_cut_off("halfway text", true)])
        .await
        .unwrap();
    let lines = sse_post(
        &format!("http://127.0.0.1:{port}/v1/chat/completions"),
        &openai_body(true),
    )
    .await
    .unwrap();
    assert!(lines.iter().any(|l| l.contains("halfway")));
    assert!(!lines.iter().any(|l| l.contains(STOP_MS)));
}

#[tokio::test]
async fn fake_model_openai_stall_keeps_connection_open() {
    let (_model, port) = model_with(vec![model_stall("thinking pause", true)])
        .await
        .unwrap();
    let outcome = timeout_real(
        sse_wait_for_line(
            &format!("http://127.0.0.1:{port}/v1/chat/completions"),
            "thinking",
            Duration::from_secs(2),
        ),
        Duration::from_secs(20),
    )
    .await
    .unwrap();
    let lines = outcome.unwrap_or_default();
    assert!(lines.iter().any(|l| l.contains("thinking")));
}

#[tokio::test]
async fn fake_model_openai_no_response_silence_times_out() {
    let (_model, port) = model_with(vec![model_no_response(true)]).await.unwrap();
    let outcome = timeout_real(
        sse_wait_for_line(
            &format!("http://127.0.0.1:{port}/v1/chat/completions"),
            "zzz-absent",
            Duration::from_secs(1),
        ),
        Duration::from_secs(20),
    )
    .await
    .unwrap();
    assert!(outcome.is_err());
}

#[tokio::test]
async fn fake_model_reply_status_returns_that_http_code() {
    let (_model, port) = model_with(vec![model_reply_status(429, "too fast")])
        .await
        .unwrap();
    let (code, body) = test_post(port, "/v1/messages", &anthropic_body(false))
        .await
        .unwrap();
    assert_eq!(code, 429);
    assert!(body.contains("too fast"));
}

#[tokio::test]
async fn fake_model_records_requests_with_body_and_header_presence() {
    let (model, port) = model_with(vec![model_replies_text("x", false)])
        .await
        .unwrap();
    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://127.0.0.1:{port}/v1/chat/completions"))
        .header("Authorization", "Bearer secret-token")
        .header("x-api-key", "key-value")
        .body(openai_body(false))
        .send()
        .await
        .unwrap();
    let code = response.status().as_u16();
    assert_eq!(code, 200);
    let recorded = model.recorded();
    assert_eq!(recorded.len(), 1);
    let first = recorded.first().unwrap();
    assert_eq!(first.path, "/v1/chat/completions");
    assert!(first.body.contains("gpt-test"));
    let auth = first
        .headers
        .iter()
        .find(|(name, _v)| name == "authorization");
    let api_key = first.headers.iter().find(|(name, _v)| name == "x-api-key");
    assert!(matches!(auth, Some((_n, HeaderValueKind::Present))));
    assert!(matches!(api_key, Some((_n, HeaderValueKind::Present))));
}

#[tokio::test]
async fn fake_model_empty_script_returns_bad_request_code() {
    let (model, port) = model_with(Vec::new()).await.unwrap();
    assert_eq!(model.script_len(), 0);
    let (code, body) = test_post(port, "/v1/messages", &anthropic_body(false))
        .await
        .unwrap();
    assert_eq!(code, 400);
    assert!(body.contains("model-script-empty"));
}

#[tokio::test]
async fn fake_model_script_consumes_steps_in_order() {
    let (model, port) = model_with(vec![
        model_replies_text("first", false),
        model_replies_text("second", false),
    ])
    .await
    .unwrap();
    let (_c1, b1) = test_post(port, "/v1/chat/completions", &openai_body(false))
        .await
        .unwrap();
    let (_c2, b2) = test_post(port, "/v1/chat/completions", &openai_body(false))
        .await
        .unwrap();
    assert!(b1.contains("first"));
    assert!(b2.contains("second"));
    assert_eq!(model.script_len(), 0);
    let _step: Step = model_replies_text("unused", false);
}
