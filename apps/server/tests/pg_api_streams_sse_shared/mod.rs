use serde_json::{Value, json};

use anybot_app_test::clockwalk::ClockWalker;
use anybot_app_test::http::ResponseSpec;
use anybot_app_test::http::{RequestSpec, post};
use anybot_app_test::sse::{SseOpen, open_sse};

pub(crate) const DOLPHIN_TEXT: &str = "\u{1F426} dolphin 42";

pub(crate) fn item(device: &str, kind: &str, content: &Value) -> Value {
    json!({
        "device_id": device,
        "kind": kind,
        "content": content,
        "created_at_ms": 1_700_000_000_100_i64,
        "offline_sent": false,
    })
}

pub(crate) fn append_body(items: &[Value]) -> String {
    json!({"stream": "account", "items": items}).to_string()
}

pub(crate) async fn login_bearer(
    served: &anybot_app_test::serve::Served,
) -> Result<String, String> {
    let base = served.base_url();
    anybot_app_test::serve::boot_pair(served).await?;
    let body = json!({
        "account": "alice",
        "password": "alicepassword",
        "label": "test-device",
        "credential": "bearer",
    })
    .to_string();
    let spec = RequestSpec::new(&body).with_key("k-sse-login");
    let mut walker = ClockWalker::start(&served.clock);
    let answer = post(&base, "/v1/account/login", &spec).await;
    walker.stop().await?;
    answer?.bearer()
}

pub(crate) async fn append(
    base: &str,
    token: &str,
    key: &str,
    body: &str,
) -> Result<ResponseSpec, String> {
    let spec = RequestSpec::new(body).with_key(key).with_bearer(token);
    post(base, "/v1/streams/append", &spec).await
}

pub(crate) async fn seed(base: &str, token: &str, count: usize) -> Result<(), String> {
    for index in 0..count {
        let items = vec![item(
            &format!("dev-{index}"),
            "message",
            &json!({"text": index}),
        )];
        let answer = append(
            base,
            token,
            &format!("k-sse-seed-{index}"),
            &append_body(&items),
        )
        .await?;
        assert_eq!(answer.status, 200, "seed {index}: {}", answer.json);
    }
    Ok(())
}

pub(crate) async fn open_account(
    base: &str,
    token: &str,
    init: Option<i64>,
) -> Result<SseOpen, String> {
    let query = match init {
        Some(at) => format!("init_seq={at}"),
        None => String::new(),
    };
    open_sse(base, "/v1/streams/notifications", token, &query).await
}

pub(crate) fn progress_seq(json: &Value) -> Result<i64, String> {
    stream_row(json, "account/alice")?
        .get("seq")
        .and_then(Value::as_i64)
        .ok_or(format!("no account seq in {json}"))
}

pub(crate) fn stream_row<'a>(json: &'a Value, key: &str) -> Result<&'a Value, String> {
    let rows = json
        .get("streams")
        .and_then(Value::as_array)
        .ok_or(format!("no streams in {json}"))?;
    rows.iter()
        .find(|row| row.get("stream").and_then(Value::as_str) == Some(key))
        .ok_or(format!("no row for {key} in {json}"))
}
