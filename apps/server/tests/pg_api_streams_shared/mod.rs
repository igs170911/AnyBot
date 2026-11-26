use serde_json::{Value, json};

use anybot_app_test::clockwalk::ClockWalker;
use anybot_app_test::http::ResponseSpec;
use anybot_app_test::http::{RequestSpec, post};
use sqlx::Row;

pub(crate) const ACCOUNT: &str = "account";

pub(crate) fn item(device: &str, kind: &str, content: &Value) -> Value {
    json!({
        "device_id": device,
        "kind": kind,
        "content": content,
        "created_at_ms": 1_700_000_000_100_i64,
        "offline_sent": false,
    })
}

pub(crate) fn append_body(stream: &str, items: &[Value]) -> String {
    let selector = if stream == ACCOUNT {
        json!("account")
    } else {
        json!({"bot": {"id": stream}})
    };
    json!({"stream": selector, "items": items}).to_string()
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
    let spec = RequestSpec::new(&body).with_key("k-stream-login");
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

pub(crate) fn results_of(answer: &ResponseSpec) -> Result<Vec<Value>, String> {
    answer
        .json
        .get("results")
        .and_then(Value::as_array)
        .cloned()
        .ok_or(format!("no results in {}", answer.json))
}

pub(crate) fn result_seq(answer: &ResponseSpec, index: usize) -> Result<i64, String> {
    let rows = results_of(answer)?;
    let row = rows
        .get(index)
        .ok_or(format!("no result {index} in {}", answer.json))?;
    row.get("seq")
        .and_then(Value::as_i64)
        .ok_or(format!("no seq in {row}"))
}

pub(crate) fn result_code(answer: &ResponseSpec, index: usize) -> Result<String, String> {
    let rows = results_of(answer)?;
    let row = rows
        .get(index)
        .ok_or(format!("no result {index} in {}", answer.json))?;
    row.get("code")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or(format!("no code in {row}"))
}

pub(crate) fn result_tag(answer: &ResponseSpec, index: usize) -> Result<String, String> {
    let rows = results_of(answer)?;
    let row = rows
        .get(index)
        .ok_or(format!("no result {index} in {}", answer.json))?;
    row.get("result")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or(format!("no result tag in {row}"))
}

pub(crate) fn two_messages() -> Vec<Value> {
    vec![
        item("dev-a", "message", &json!({"text": "first"})),
        item("dev-b", "message", &json!({"text": "second"})),
    ]
}

pub(crate) async fn streams_count(served: &anybot_app_test::serve::Served) -> Result<i64, String> {
    let row = sqlx::query("SELECT count(*) AS n FROM records")
        .fetch_one(&served.pool)
        .await
        .map_err(|e| e.to_string())?;
    let n: i64 = row.get("n");
    Ok(n)
}

pub(crate) async fn login_bob(served: &anybot_app_test::serve::Served) -> Result<String, String> {
    let base = served.base_url();
    let body = json!({
        "account": "bob",
        "password": "bobpassword",
        "label": "test-device",
        "credential": "bearer",
    })
    .to_string();
    let spec = RequestSpec::new(&body).with_key("k-bob-login");
    let mut walker = ClockWalker::start(&served.clock);
    let answer = post(&base, "/v1/account/login", &spec).await;
    walker.stop().await?;
    answer?.bearer()
}

pub(crate) async fn admin_bearer(
    served: &anybot_app_test::serve::Served,
) -> Result<String, String> {
    let base = served.base_url();
    let body = json!({
        "account": "alice",
        "password": "alicepassword",
        "label": "admin-device",
        "credential": "bearer",
    })
    .to_string();
    let spec = RequestSpec::new(&body).with_key("k-admin-login");
    let mut walker = ClockWalker::start(&served.clock);
    let answer = post(&base, "/v1/account/login", &spec).await;
    walker.stop().await?;
    answer?.bearer()
}
