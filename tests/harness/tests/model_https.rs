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
use test_harness::{https_client, mint_self_signed, model_replies_text, model_start_https};
use time as _;
use tokio_rustls as _;
use tokio_stream as _;
use toml as _;
use url as _;

#[tokio::test]
async fn https_fake_model_accepts_its_own_listed_cert() -> Result<(), String> {
    let started = model_start_https("model.invalid").await?;
    started
        .model
        .script_set(vec![model_replies_text("ok", false)]);
    let client = https_client(&started.cert.pem, "model.invalid", started.port)?;
    let url = format!("https://model.invalid:{}/v1/chat/completions", started.port);
    let response = client
        .post(url)
        .body(r#"{"model":"local","messages":[]}"#)
        .send()
        .await
        .map_err(|err| format!("{err:?}"))?;
    let text = response.text().await.map_err(|err| err.to_string())?;
    assert!(text.contains("ok"), "{text}");
    let other = mint_self_signed("model.invalid", 2)?;
    assert_ne!(other.pem, started.cert.pem);
    Ok(())
}
