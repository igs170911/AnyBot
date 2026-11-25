use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::put;
use tokio::net::TcpListener;

use crate::push_crypto::{SubscriptionKeys, decrypt_aes128gcm};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct PushRecord {
    pub subscription_id: String,
    pub plaintext: Vec<u8>,
    pub ttl: Option<String>,
    pub topic: Option<String>,
    pub urgency: Option<String>,
}

#[derive(Default)]
struct PushInner {
    subs: BTreeMap<String, SubscriptionKeys>,
    records: Vec<PushRecord>,
}

#[derive(Default)]
pub struct FakePush {
    inner: Mutex<PushInner>,
}

impl FakePush {
    pub fn register(&self, subscription_id: &str, keys: SubscriptionKeys) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        inner.subs.insert(subscription_id.to_owned(), keys);
    }

    pub fn records(&self) -> Vec<PushRecord> {
        let Ok(inner) = self.inner.lock() else {
            return Vec::new();
        };
        inner.records.clone()
    }

    pub fn clear(&self) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        inner.records.clear();
    }

    fn keys(&self, subscription_id: &str) -> Option<SubscriptionKeys> {
        let Ok(inner) = self.inner.lock() else {
            return None;
        };
        inner.subs.get(subscription_id).cloned()
    }

    fn store(&self, record: PushRecord) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        inner.records.push(record);
    }
}

pub async fn push_start() -> Result<(Arc<FakePush>, u16), String> {
    let push = Arc::new(FakePush::default());
    let (std_listener, port) = crate::ports::bind_v4()?;
    std_listener
        .set_nonblocking(true)
        .map_err(|e| format!("set nonblocking failed: {e}"))?;
    let listener =
        TcpListener::from_std(std_listener).map_err(|e| format!("convert listener failed: {e}"))?;
    let router = Router::new()
        .route("/push/{id}", put(handle_push))
        .with_state(Arc::clone(&push));
    tokio::spawn(async move {
        let _served = axum::serve(listener, router).await;
    });
    Ok((push, port))
}

#[axum::debug_handler]
async fn handle_push(
    Path(id): Path<String>,
    State(push): State<Arc<FakePush>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let Some(keys) = push.keys(&id) else {
        return StatusCode::GONE.into_response();
    };
    let Ok(plaintext) = decrypt_aes128gcm(&keys, &body) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    push.store(PushRecord {
        subscription_id: id,
        plaintext,
        ttl: header_text(&headers, "ttl"),
        topic: header_text(&headers, "topic"),
        urgency: header_text(&headers, "urgency"),
    });
    StatusCode::ACCEPTED.into_response()
}

fn header_text(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(String::from)
}

#[must_use]
pub fn push_notify_body(title: &str, navigate: &str) -> String {
    serde_json::json!({ "title": title, "navigate": navigate }).to_string()
}
