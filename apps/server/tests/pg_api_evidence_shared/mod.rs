use serde_json::{Value, json};

use anybot_app_test::clockwalk::ClockWalker;
use anybot_app_test::http::{RequestSpec, ResponseSpec, get, post};
use anybot_app_test::serve::{Served, boot_pair};
use sqlx::Row;

pub(crate) const BOT: &str = "bot-e1";
pub(crate) const TASK: &str = "task-e1";
pub(crate) const FIRST_MESSAGE: &str = "018f0000-0000-7000-8000-0000000000f1";
pub(crate) const SECOND_MESSAGE: &str = "018f0000-0000-7000-8000-0000000000f2";
pub(crate) const TOOL_RECORD: &str = "018f0000-0000-7000-8000-0000000000f3";
pub(crate) const EVIDENCE_RECORD: &str = "018f0000-0000-7000-8000-0000000000f4";

pub(crate) fn record(device: &str, kind: &str, content: &Value, task: Option<&str>) -> Value {
    let mut value = json!({
        "device_id": device,
        "kind": kind,
        "content": content,
        "created_at_ms": 1_700_000_000_100_i64,
        "offline_sent": false,
    });
    if let (Some(task), Some(map)) = (task, value.as_object_mut()) {
        map.insert("task_id".to_owned(), json!(task));
    }
    value
}

pub(crate) async fn login(served: &Served) -> Result<String, String> {
    boot_pair(served).await?;
    let body = json!({
        "account": "alice",
        "password": "alicepassword",
        "label": "test-device",
        "credential": "bearer",
    })
    .to_string();
    let spec = RequestSpec::new(&body).with_key("k-evidence-login");
    let mut walker = ClockWalker::start(&served.clock);
    let answer = post(&served.base_url(), "/v1/account/login", &spec).await;
    walker.stop().await?;
    answer?.bearer()
}

pub(crate) async fn send(
    served: &Served,
    token: &str,
    key: &str,
    stream: &Value,
    items: &[Value],
) -> Result<ResponseSpec, String> {
    let body = json!({"stream": stream, "items": items}).to_string();
    let spec = RequestSpec::new(&body).with_key(key).with_bearer(token);
    post(&served.base_url(), "/v1/streams/append", &spec).await
}

pub(crate) fn bot_stream() -> Value {
    json!({"bot": {"id": BOT}})
}

pub(crate) async fn create_bot(served: &Served, token: &str) -> Result<(), String> {
    let create = record(
        "018f0000-0000-7000-8000-0000000000f0",
        "bots_create",
        &json!({"bot_id": BOT, "name": "helper"}),
        None,
    );
    let landed = send(
        served,
        token,
        "k-evidence-bot",
        &json!("account"),
        &[create],
    )
    .await?;
    expect_accepted(&landed, 0)?;
    Ok(())
}

pub(crate) fn expect_accepted(answer: &ResponseSpec, index: usize) -> Result<i64, String> {
    let row = result_row(answer, index)?;
    if row.get("result").and_then(Value::as_str) != Some("accepted") {
        return Err(format!("not accepted: {}", answer.json));
    }
    row.get("seq")
        .and_then(Value::as_i64)
        .ok_or(format!("no seq: {}", answer.json))
}

pub(crate) fn result_row(answer: &ResponseSpec, index: usize) -> Result<&Value, String> {
    answer
        .json
        .get("results")
        .and_then(Value::as_array)
        .and_then(|rows| rows.get(index))
        .ok_or(format!("no result {index} in {}", answer.json))
}

pub(crate) async fn act(
    served: &Served,
    token: &str,
    key: &str,
    action: &str,
    target: Option<i64>,
) -> Result<ResponseSpec, String> {
    let body = match target {
        Some(seq) => json!({"stream": bot_stream(), "action": action, "target_seq": seq}),
        None => json!({"stream": bot_stream(), "action": action}),
    };
    let spec = RequestSpec::new(&body.to_string())
        .with_key(key)
        .with_bearer(token);
    post(&served.base_url(), "/v1/streams/actions", &spec).await
}

pub(crate) async fn fetch_record(
    served: &Served,
    token: &str,
    seq: i64,
) -> Result<ResponseSpec, String> {
    let path = format!("/v1/streams/record?stream=bot/{BOT}&seq={seq}");
    get(
        &served.base_url(),
        &path,
        &RequestSpec::new("").with_bearer(token),
    )
    .await
}

pub(crate) async fn record_count(served: &Served) -> Result<i64, String> {
    let row = sqlx::query("SELECT count(*) AS n FROM records")
        .fetch_one(&served.pool)
        .await
        .map_err(|e| e.to_string())?;
    row.try_get::<i64, _>("n").map_err(|e| e.to_string())
}

pub(crate) async fn stored(served: &Served, device: &str) -> Result<(String, bool), String> {
    let row = sqlx::query("SELECT content, purged FROM records WHERE device_id = $1")
        .bind(device)
        .fetch_one(&served.pool)
        .await
        .map_err(|e| e.to_string())?;
    let content: String = row.try_get("content").map_err(|e| e.to_string())?;
    let purged: bool = row.try_get("purged").map_err(|e| e.to_string())?;
    Ok((content, purged))
}

pub(crate) fn conversation(secret: &str) -> Vec<Value> {
    vec![
        record(FIRST_MESSAGE, "message", &json!({"text": secret}), None),
        record(
            SECOND_MESSAGE,
            "message",
            &json!({"text": "second line"}),
            None,
        ),
        record(
            TOOL_RECORD,
            "tool_completed",
            &json!({"call_id": "call-1", "summary": "listed the folder", "raw_id": null}),
            Some(TASK),
        ),
        record(
            EVIDENCE_RECORD,
            "evidence_added",
            &json!({"short": 1, "tool": "shell", "kind": "executed", "source_seq": 1}),
            Some(TASK),
        ),
    ]
}
