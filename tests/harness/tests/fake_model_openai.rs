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
use tracing as _;
use tracing_subscriber as _;
use url as _;

use std::sync::Arc;
use std::time::Duration;

use test_harness::fake_model::{ModelFormat, model_start_with_format};
use test_harness::http::{sse_post, test_post};
use test_harness::model_script::{
    Step, model_calls_tool, model_replies_text, model_think_then_reply,
};
use test_harness::realtime::timeout_real;

const STOP_MS: &str = "[DONE]";

fn at<'a>(value: &'a serde_json::Value, key: &str) -> &'a serde_json::Value {
    value.get(key).unwrap_or(&serde_json::Value::Null)
}

fn at_index(value: &serde_json::Value, idx: usize) -> &serde_json::Value {
    &value[idx]
}

fn choice0(value: &serde_json::Value) -> &serde_json::Value {
    at_index(at(value, "choices"), 0)
}

fn openai_body(stream: bool) -> String {
    serde_json::json!({ "model": "fake", "messages": [], "stream": stream }).to_string()
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
async fn fake_model_openai_json_official_fields_only() {
    let (_model, port) = model_with(vec![model_replies_text("plain", false)])
        .await
        .unwrap();
    let (code, body) = test_post(port, "/v1/chat/completions", &openai_body(false))
        .await
        .unwrap();
    assert_eq!(code, 200);
    let value: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(at(&value, "object").as_str().unwrap(), "chat.completion");
    let message = at(choice0(&value), "message");
    assert_eq!(at(message, "content").as_str().unwrap(), "plain");
    for field in [
        "reasoning",
        "routed_experts",
        "weight_versions",
        "prompt_logprobs",
        "kv_transfer_params",
        "token_ids",
        "stop_reason",
        "prompt_text",
        "prompt_token_ids",
        "ec_transfer_params",
        "metrics",
    ] {
        assert!(
            value.get(field).is_none(),
            "openai format must lack {field}"
        );
        assert!(
            choice0(&value).get(field).is_none(),
            "choice must lack {field}"
        );
        assert!(message.get(field).is_none(), "message must lack {field}");
    }
}

#[tokio::test]
async fn fake_model_openai_stream_has_no_server_fields() {
    let (_model, port) = model_with(vec![model_think_then_reply("ponder", "answer", true)])
        .await
        .unwrap();
    let lines = sse_post(
        &format!("http://127.0.0.1:{port}/v1/chat/completions"),
        &openai_body(true),
    )
    .await
    .unwrap();
    assert!(lines.iter().any(|l| l.contains("answer")));
    assert!(lines.iter().any(|l| l.contains(STOP_MS)));
    assert!(!lines.iter().any(|l| l.contains("\"reasoning\"")));
    assert!(lines.iter().any(|l| l.contains("reasoning_content")));
    assert!(!lines.iter().any(|l| l.contains("token_ids")));
    assert!(!lines.iter().any(|l| l.contains("kv_transfer_params")));
}

#[tokio::test]
async fn fake_model_openai_tool_call_stream_finish_reason_is_tool_calls() {
    let (_model, port) = model_with(vec![model_calls_tool(
        "weather",
        serde_json::json!({}),
        true,
    )])
    .await
    .unwrap();
    let lines = sse_post(
        &format!("http://127.0.0.1:{port}/v1/chat/completions"),
        &openai_body(true),
    )
    .await
    .unwrap();
    assert!(lines.iter().any(|l| l.contains("tool_calls")));
    assert!(
        lines
            .iter()
            .any(|l| l.contains("\"finish_reason\":\"tool_calls\""))
    );
}
