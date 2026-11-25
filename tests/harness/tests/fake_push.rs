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
use serde as _;
use serde_json as _;
use sha2 as _;
use socket2 as _;
use sqlx as _;
use tokio_stream as _;
use toml as _;
use url as _;

use std::time::Duration;

use sad_web_push::{ContentEncoding, SubscriptionInfo, Urgency, WebPushMessageBuilder};

use test_harness::fake_push::{PushRecord, push_start};
use test_harness::push_crypto::{SubscriptionKeys, decrypt_aes128gcm, subscription_endpoint};
use test_harness::realtime::timeout_real;

const PLAIN: &[u8] = b"push me to the edge";

async fn put_payload(port: u16, id: &str, keys: &SubscriptionKeys) -> Result<u16, String> {
    let info = SubscriptionInfo::new(
        subscription_endpoint(port, id),
        keys.public_b64(),
        keys.auth_b64(),
    );
    let mut builder = WebPushMessageBuilder::new(&info);
    builder.allow_insecure_endpoint(true);
    builder.set_ttl(60);
    builder.set_topic("alerts".to_owned());
    builder.set_urgency(Urgency::High);
    builder.set_payload(ContentEncoding::Aes128Gcm, PLAIN);
    let message = builder
        .build()
        .map_err(|e| format!("push build failed: {e}"))?;
    let payload = message
        .payload
        .ok_or("push message has no payload".to_owned())?;
    let mut reqwest_builder = reqwest::Client::new()
        .put(message.endpoint.to_string())
        .header("ttl", message.ttl.to_string())
        .header("content-encoding", payload.content_encoding.to_str())
        .body(payload.content);
    if let Some(topic) = message.topic {
        reqwest_builder = reqwest_builder.header("topic", topic);
    }
    if let Some(urgency) = message.urgency {
        reqwest_builder = reqwest_builder.header("urgency", urgency.to_string());
    }
    for (name, value) in payload.crypto_headers {
        reqwest_builder = reqwest_builder.header(name, value);
    }
    reqwest_builder
        .send()
        .await
        .map_err(|e| format!("push PUT failed: {e}"))
        .map(|r| r.status().as_u16())
}

#[tokio::test]
async fn fake_push_decrypts_sender_payload_byte_for_byte() {
    let keys = SubscriptionKeys::generate().unwrap();
    let (push, port) = timeout_real(push_start(), Duration::from_secs(20))
        .await
        .unwrap()
        .unwrap();
    push.register("sub-one", keys.clone());
    let code = put_payload(port, "sub-one", &keys).await.unwrap();
    assert_eq!(code, 202);
    let records = push.records();
    let first = records.first().cloned().unwrap();
    assert_eq!(first.subscription_id, "sub-one");
    assert_eq!(first.plaintext, PLAIN);
    assert_eq!(first.ttl.as_deref(), Some("60"));
    assert_eq!(first.topic.as_deref(), Some("alerts"));
    assert_eq!(first.urgency.as_deref(), Some("high"));
}

#[tokio::test]
async fn push_crypto_probe_decrypt_round_trip() {
    let keys = SubscriptionKeys::generate().unwrap();
    let info = SubscriptionInfo::new(
        subscription_endpoint(9, "probe"),
        keys.public_b64(),
        keys.auth_b64(),
    );
    let mut builder = WebPushMessageBuilder::new(&info);
    builder.allow_insecure_endpoint(true);
    builder.set_payload(ContentEncoding::Aes128Gcm, PLAIN);
    let message = builder.build().unwrap();
    let payload = message.payload.unwrap();
    let outcome = decrypt_aes128gcm(&keys, &payload.content);
    assert_eq!(outcome.unwrap(), PLAIN.to_vec());
}

#[tokio::test]
async fn fake_push_unknown_subscription_is_gone() {
    let (push, port) = timeout_real(push_start(), Duration::from_secs(20))
        .await
        .unwrap()
        .unwrap();
    let keys = SubscriptionKeys::generate().unwrap();
    let code = put_payload(port, "nobody", &keys).await.unwrap();
    assert_eq!(code, 410);
    assert_eq!(push.records().len(), 0);
}

#[tokio::test]
async fn fake_push_records_multiple_and_clears() {
    let keys = SubscriptionKeys::generate().unwrap();
    let (push, port) = timeout_real(push_start(), Duration::from_secs(20))
        .await
        .unwrap()
        .unwrap();
    push.register("sub-two", keys.clone());
    for _round in 0..2 {
        let code = put_payload(port, "sub-two", &keys).await.unwrap();
        assert_eq!(code, 202);
    }
    assert_eq!(push.records().len(), 2);
    push.clear();
    let empty: Vec<PushRecord> = Vec::new();
    assert_eq!(push.records(), empty);
}
