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
use reqwest as _;
use sad_web_push as _;
use serde as _;
use serde_json as _;
use sha2 as _;
use socket2 as _;
use sqlx as _;
use tokio_stream as _;
use toml as _;
use url as _;

use std::sync::Arc;
use std::time::Duration;

use test_harness::fake_model::{ModelFormat, model_start_with_format};
use test_harness::http::{sse_post, test_post};
use test_harness::model_script::{Step, model_calls_tool, model_think_then_reply};
use test_harness::realtime::timeout_real;

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
async fn fake_model_vllm_reasoning_stream_uses_reasoning_field() {
    let (_model, port) = model_with_format(
        vec![model_think_then_reply("ponder", "answer", true)],
        ModelFormat::Vllm,
    )
    .await
    .unwrap();
    let lines = sse_post(
        &format!("http://127.0.0.1:{port}/v1/chat/completions"),
        &openai_body(true),
    )
    .await
    .unwrap();
    assert!(lines.iter().any(|l| l.contains("\"reasoning\"")));
    assert!(lines.iter().any(|l| l.contains("ponder")));
    assert!(lines.iter().any(|l| l.contains("answer")));
}

#[tokio::test]
async fn fake_model_vllm_json_tool_call_finish_stop_and_server_fields() {
    let (_model, port) = model_with_format(
        vec![model_calls_tool("weather", serde_json::json!({}), false)],
        ModelFormat::Vllm,
    )
    .await
    .unwrap();
    let (code, body) = test_post(port, "/v1/chat/completions", &openai_body(false))
        .await
        .unwrap();
    assert_eq!(code, 200);
    let value: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        choice0(&value)
            .get("finish_reason")
            .unwrap()
            .as_str()
            .unwrap(),
        "stop"
    );
    let call = at_index(at(at(choice0(&value), "message"), "tool_calls"), 0);
    assert_eq!(at(call, "id").as_str().unwrap(), "call_fake");
    for field in [
        "routed_experts",
        "weight_versions",
        "stop_reason",
        "token_ids",
    ] {
        assert!(choice0(&value).get(field).is_some(), "missing {field}");
    }
    for field in [
        "prompt_logprobs",
        "kv_transfer_params",
        "ec_transfer_params",
        "metrics",
    ] {
        assert!(value.get(field).is_some(), "missing {field}");
    }
    assert!(
        at(choice0(&value).get("message").unwrap(), "reasoning").is_string(),
        "reasoning must be a string"
    );
}

#[tokio::test]
async fn fake_model_vllm_stream_tool_call_finish_reason_is_stop() {
    let (_model, port) = model_with_format(
        vec![model_calls_tool("weather", serde_json::json!({}), true)],
        ModelFormat::Vllm,
    )
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
            .any(|l| l.contains("\"finish_reason\":\"stop\""))
    );
    assert!(
        !lines
            .iter()
            .any(|l| l.contains("\"finish_reason\":\"tool_calls\""))
    );
}
