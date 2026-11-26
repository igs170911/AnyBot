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

use std::time::Duration;

use test_harness::fake_model::{ModelFormat, model_start_with_format};
use test_harness::http::{sse_post, test_get_async, test_post};
use test_harness::model_script::{
    ErrorBodyKind, KeyMatch, ToolCallScript, UsageScript, model_empty_body, model_error_body,
    model_list, model_list_silent, model_redacted_thinking, model_redirect,
    model_reply_status_retry, model_stop_reason, model_stream_error, model_thinking_block,
    model_tool_calls, model_usage,
};
use test_harness::realtime::timeout_real;

async fn started(
    format: ModelFormat,
) -> Result<(std::sync::Arc<test_harness::FakeModel>, u16), String> {
    let started = timeout_real(model_start_with_format(format), Duration::from_secs(20)).await??;
    Ok(started)
}

#[tokio::test]
async fn fake_model_key_match_records_three_outcomes_without_the_value() -> Result<(), String> {
    let (model, port) = started(ModelFormat::OpenAi).await?;
    model.expect_key("k1");
    model.script_set(vec![
        test_harness::model_replies_text("a", false),
        test_harness::model_replies_text("b", false),
        test_harness::model_replies_text("c", false),
    ]);
    let url = format!("http://127.0.0.1:{port}/v1/chat/completions");
    let client = reqwest::Client::new();
    let body = r#"{"model":"m","messages":[]}"#;
    client
        .post(&url)
        .header("Authorization", "Bearer k1")
        .body(body)
        .send()
        .await
        .map_err(|err| format!("{err:?}"))?;
    client
        .post(&url)
        .header("Authorization", "Bearer other")
        .body(body)
        .send()
        .await
        .map_err(|err| format!("{err:?}"))?;
    client
        .post(&url)
        .body(body)
        .send()
        .await
        .map_err(|err| format!("{err:?}"))?;
    let recorded = model.recorded();
    assert_eq!(recorded.len(), 3);
    let first = recorded.first().ok_or("first")?;
    let second = recorded.get(1).ok_or("second")?;
    let third = recorded.get(2).ok_or("third")?;
    assert_eq!(first.key_match, KeyMatch::Match);
    assert_eq!(second.key_match, KeyMatch::Mismatch);
    assert_eq!(third.key_match, KeyMatch::Missing);
    let dumped = format!("{recorded:?}");
    assert!(!dumped.contains("k1"));
    assert!(!dumped.contains("other"));
    Ok(())
}

#[tokio::test]
async fn fake_model_error_carries_retry_after_and_provider_shape() -> Result<(), String> {
    let (model, port) = started(ModelFormat::Anthropic).await?;
    model.script_set(vec![model_reply_status_retry(429, "slow down", 8)]);
    let (code, body) = test_post(port, "/v1/messages", "{}").await?;
    assert_eq!(code, 429);
    assert!(body.contains("slow down"));
    let (model, port) = started(ModelFormat::OpenAi).await?;
    model.script_set(vec![model_error_body(
        400,
        "maximum context length is 50000",
        ErrorBodyKind::OpenAi,
    )]);
    let (_code, openai) = test_post(port, "/v1/chat/completions", "{}").await?;
    assert!(openai.contains("context_length_exceeded"));
    let (model, port) = started(ModelFormat::Anthropic).await?;
    model.script_set(vec![model_error_body(
        400,
        "prompt is too long: 50000",
        ErrorBodyKind::Anthropic,
    )]);
    let (_code, claude) = test_post(port, "/v1/messages", "{}").await?;
    assert!(claude.contains("invalid_request_error"));
    let (model, port) = started(ModelFormat::Vllm).await?;
    model.script_set(vec![model_error_body(
        400,
        "maximum context length is 50000",
        ErrorBodyKind::Vllm,
    )]);
    let (_code, vllm) = test_post(port, "/v1/chat/completions", "{}").await?;
    assert!(vllm.contains("BadRequestError"));
    Ok(())
}

#[tokio::test]
async fn fake_model_stream_error_busy_and_other() -> Result<(), String> {
    let (model, port) = started(ModelFormat::OpenAi).await?;
    model.script_set(vec![model_stream_error(true, "engine is busy")]);
    let lines = sse_post(
        &format!("http://127.0.0.1:{port}/v1/chat/completions"),
        r#"{"stream":true}"#,
    )
    .await?;
    let joined = lines.join("\n");
    assert!(joined.contains("engine_busy"));
    let (model, port) = started(ModelFormat::Anthropic).await?;
    model.script_set(vec![model_stream_error(false, "broke")]);
    let lines = sse_post(
        &format!("http://127.0.0.1:{port}/v1/messages"),
        r#"{"stream":true}"#,
    )
    .await?;
    assert!(lines.join("\n").contains("api_error"));
    Ok(())
}

#[tokio::test]
async fn fake_model_usage_thinking_refusal_and_tools() -> Result<(), String> {
    let (model, port) = started(ModelFormat::Anthropic).await?;
    model.script_set(vec![
        model_usage(
            "ok",
            UsageScript {
                input_tokens: 1000,
                cache_read_tokens: 5000,
                cache_write_tokens: 200,
                output_tokens: 50,
                thinking_tokens: 7,
            },
            false,
        ),
        model_thinking_block("plan", "sig-1", "done", false),
        model_redacted_thinking("opaque", "done", false),
        model_stop_reason("no", "refusal", false),
        model_stop_reason("cut", "max_tokens", false),
    ]);
    let (_c, usage) = test_post(port, "/v1/messages", "{}").await?;
    assert!(usage.contains("\"cache_read_input_tokens\":5000"));
    let (_c, signed) = test_post(port, "/v1/messages", "{}").await?;
    assert!(signed.contains("sig-1"));
    let (_c, redacted) = test_post(port, "/v1/messages", "{}").await?;
    assert!(redacted.contains("redacted_thinking"));
    let (_c, refused) = test_post(port, "/v1/messages", "{}").await?;
    assert!(refused.contains("refusal"));
    let (_c, limited) = test_post(port, "/v1/messages", "{}").await?;
    assert!(limited.contains("max_tokens"));
    let (model, port) = started(ModelFormat::OpenAi).await?;
    let mut first = ToolCallScript::named("read_file", "");
    first.id = None;
    first.argument_parts = vec!["{\"path\":".to_owned(), "\"a.txt\"}".to_owned()];
    let second = ToolCallScript::named("echo", "{\"x\":1}");
    model.script_set(vec![model_tool_calls(
        vec![first, second],
        "tool_calls",
        true,
    )]);
    let lines = sse_post(
        &format!("http://127.0.0.1:{port}/v1/chat/completions"),
        r#"{"stream":true}"#,
    )
    .await?;
    let joined = lines.join("\n");
    assert!(joined.contains("read_file"));
    assert!(joined.contains("echo"));
    assert!(joined.contains("a.txt"));
    Ok(())
}

#[tokio::test]
async fn fake_model_empty_redirect_and_model_list() -> Result<(), String> {
    let (model, port) = started(ModelFormat::OpenAi).await?;
    model.script_set(vec![
        model_empty_body(),
        model_redirect("http://api.cloud.test/v1"),
    ]);
    let (code, body) = test_post(port, "/v1/chat/completions", "{}").await?;
    assert_eq!(code, 200);
    assert!(body.is_empty());
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|err| format!("{err:?}"))?;
    let redirected = client
        .post(format!("http://127.0.0.1:{port}/v1/chat/completions"))
        .body("{}")
        .send()
        .await
        .map_err(|err| format!("{err:?}"))?;
    assert_eq!(redirected.status().as_u16(), 302);
    assert_eq!(
        redirected
            .headers()
            .get("location")
            .and_then(|v| v.to_str().ok()),
        Some("http://api.cloud.test/v1")
    );
    model.script_set(vec![
        model_list(vec!["models/b".to_owned(), "a".to_owned()]),
        model_list(Vec::new()),
        model_list_silent(),
    ]);
    let (code, listed) = test_get_async(port, "/v1/models").await?;
    assert_eq!(code, 200);
    assert!(listed.contains("models/b"));
    let (code, empty) = test_get_async(port, "/v1/models").await?;
    assert_eq!(code, 200);
    assert!(empty.contains("\"data\":[]"));
    Ok(())
}
