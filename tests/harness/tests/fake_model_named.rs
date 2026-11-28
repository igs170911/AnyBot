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
use test_harness::model_replies_text;
use test_harness::realtime::timeout_real;

const DEADLINE: Duration = Duration::from_secs(20);

async fn ask(port: u16, model: &str) -> Result<String, String> {
    let url = format!("http://127.0.0.1:{port}/v1/chat/completions");
    let body = serde_json::json!({"model": model, "stream": false, "messages": []});
    let reply = timeout_real(
        reqwest::Client::new().post(url).json(&body).send(),
        DEADLINE,
    )
    .await?
    .map_err(|e| e.to_string())?;
    let status = reply.status().as_u16();
    let text = reply.text().await.map_err(|e| e.to_string())?;
    Ok(format!("{status}:{text}"))
}

#[tokio::test]
async fn named_scripts_keep_separate_queues_per_model() -> Result<(), String> {
    let (model, port) =
        timeout_real(model_start_with_format(ModelFormat::OpenAi), DEADLINE).await??;
    model.script_set_for("fake-a", vec![model_replies_text("from a", false)]);
    model.script_set_for("fake-b", vec![model_replies_text("from b", false)]);
    let from_b = ask(port, "fake-b").await?;
    let from_a = ask(port, "fake-a").await?;
    assert!(from_a.contains("from a"), "{from_a}");
    assert!(from_b.contains("from b"), "{from_b}");
    assert_eq!(model.script_len_for("fake-a"), 0);
    assert_eq!(model.script_len_for("fake-b"), 0);
    assert_eq!(model.recorded_for("fake-a").len(), 1);
    assert_eq!(model.recorded_for("fake-b").len(), 1);
    Ok(())
}

#[tokio::test]
async fn named_queue_is_not_refilled_from_the_shared_script() -> Result<(), String> {
    let (model, port) =
        timeout_real(model_start_with_format(ModelFormat::OpenAi), DEADLINE).await??;
    model.script_set(vec![model_replies_text("shared", false)]);
    model.script_set_for("fake-a", Vec::new());
    let named = ask(port, "fake-a").await?;
    assert!(named.starts_with("400:"), "{named}");
    let shared = ask(port, "other").await?;
    assert!(shared.contains("shared"), "{shared}");
    Ok(())
}
