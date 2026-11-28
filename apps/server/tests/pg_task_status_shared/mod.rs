use anybot_app_test::clockwalk::ClockWalker;
use anybot_app_test::http::{RequestSpec, ResponseSpec, get, post};
use anybot_app_test::serve::Served;
use anybot_foundation::clock::SystemClock;
use anybot_foundation::{BotId, DeviceId, TaskId};
use serde_json::{Value, json};

pub(crate) const ACCOUNT: &str = "account";

pub(crate) fn dev_uuid() -> String {
    DeviceId::generate(&SystemClock).as_str()
}

pub(crate) fn bot_uuid() -> String {
    BotId::generate(&SystemClock).as_str()
}

pub(crate) fn task_uuid() -> String {
    TaskId::generate(&SystemClock).as_str()
}

pub(crate) fn item(device: &str, kind: &str, content: &Value) -> Value {
    json!({
        "device_id": device,
        "kind": kind,
        "content": content,
        "created_at_ms": 1_700_000_000_100_i64,
        "offline_sent": false,
    })
}

pub(crate) fn task_event_item(
    device: &str,
    kind: &str,
    task: &str,
    data: &Value,
    at_ms: i64,
) -> Value {
    let mut row = item(device, kind, data);
    if let Some(map) = row.as_object_mut() {
        map.insert("created_at_ms".to_owned(), json!(at_ms));
        map.insert("task_id".to_owned(), json!(task));
    }
    row
}

pub(crate) fn bot_create(bot: &str) -> Vec<Value> {
    vec![item(
        &dev_uuid(),
        "bots_create",
        &json!({"bot_id": bot, "name": "status bot"}),
    )]
}

pub(crate) fn answer(device: &str, task: &str, text: &str) -> Vec<Value> {
    let created = json!({
        "goal": text, "opened_by": "you", "bot": task_bot_ref(device), "parent": null
    });
    vec![
        task_event_item(device, "task_created", task, &created, 1_700_000_000_100),
        model_answer_row(&dev_uuid(), task, text, 1_700_000_000_200),
    ]
}

fn task_bot_ref(device: &str) -> String {
    match BotId::parse(device) {
        Ok(id) => id.as_str(),
        Err(_) => BotId::generate(&SystemClock).as_str(),
    }
}

fn model_answer_row(device: &str, task: &str, text: &str, at_ms: i64) -> Value {
    task_event_item(
        device,
        "model_answer",
        task,
        &json!({
            "text": text,
            "thinking": null,
            "tool_requests": [],
            "finish": "finished",
            "native": null
        }),
        at_ms,
    )
}

pub(crate) fn lifecycle(task: &str, bot: &str, text: &str) -> Vec<Value> {
    let created = json!({
        "goal": text, "opened_by": "you", "bot": bot, "parent": null
    });
    let ended = json!({ "reason": { "kind": "answered" }, "reply_delivered": true });
    let report = json!({ "kind": "shown", "file": "report.txt" });
    vec![
        task_event_item(
            &dev_uuid(),
            "task_created",
            task,
            &created,
            1_700_000_000_100,
        ),
        model_answer_row(&dev_uuid(), task, text, 1_700_000_000_200),
        task_event_item(&dev_uuid(), "turn_ended", task, &ended, 1_700_000_000_300),
        task_event_item(
            &dev_uuid(),
            "device_report",
            task,
            &report,
            1_700_000_000_400,
        ),
    ]
}

pub(crate) fn thumbs(task: &str, answer_seq: i64, device: &str, kind: &str) -> Vec<Value> {
    thumbs_at(task, answer_seq, device, kind, 1_700_000_000_500)
}

pub(crate) fn thumbs_at(
    task: &str,
    answer_seq: i64,
    device: &str,
    kind: &str,
    at_ms: i64,
) -> Vec<Value> {
    vec![task_event_item(
        device,
        kind,
        task,
        &json!({"answer_seq": answer_seq}),
        at_ms,
    )]
}

pub(crate) fn append_body(stream: &str, items: &[Value]) -> String {
    let selector = if stream == ACCOUNT {
        json!("account")
    } else {
        json!({"bot": {"id": stream}})
    };
    json!({"stream": selector, "items": items}).to_string()
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

pub(crate) async fn login_bearer(served: &Served) -> Result<String, String> {
    let base = served.base_url();
    anybot_app_test::serve::boot_pair(served).await?;
    let body = json!({
        "account": "alice",
        "password": "alicepassword",
        "label": "test-device",
        "credential": "bearer",
    })
    .to_string();
    let spec = RequestSpec::new(&body).with_key("k-status-login");
    let mut walker = ClockWalker::start(&served.clock);
    let answer = post(&base, "/v1/account/login", &spec).await;
    walker.stop().await?;
    answer?.bearer()
}

pub(crate) async fn login_bob(served: &Served) -> Result<String, String> {
    let base = served.base_url();
    let body = json!({
        "account": "bob",
        "password": "bobpassword",
        "label": "test-device",
        "credential": "bearer",
    })
    .to_string();
    let spec = RequestSpec::new(&body).with_key("k-status-bob");
    let mut walker = ClockWalker::start(&served.clock);
    let answer = post(&base, "/v1/account/login", &spec).await;
    walker.stop().await?;
    answer?.bearer()
}

fn result_row(answer: &ResponseSpec, index: usize) -> Result<Value, String> {
    let rows = answer
        .json
        .get("results")
        .and_then(Value::as_array)
        .ok_or(format!("no results in {}", answer.json))?;
    rows.get(index)
        .cloned()
        .ok_or(format!("no result {index} in {}", answer.json))
}

pub(crate) fn result_tag(answer: &ResponseSpec, index: usize) -> Result<String, String> {
    let row = result_row(answer, index)?;
    row.get("result")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or(format!("no result tag in {row}"))
}

pub(crate) fn result_seq(answer: &ResponseSpec, index: usize) -> Result<i64, String> {
    let row = result_row(answer, index)?;
    row.get("seq")
        .and_then(Value::as_i64)
        .ok_or(format!("no seq in {row}"))
}

pub(crate) fn result_code(answer: &ResponseSpec, index: usize) -> Result<String, String> {
    let row = result_row(answer, index)?;
    row.get("code")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or(format!("no code in {row}"))
}

pub(crate) fn value_at(json: &Value, keys: &[&str]) -> Option<Value> {
    let mut current = json.clone();
    for key in keys {
        current = current.get(*key).cloned()?;
    }
    Some(current)
}

fn status_path(task: &str, bot: &str) -> String {
    format!("/v1/tasks/{task}/status?bot=bot/{bot}")
}

pub(crate) async fn status(
    base: &str,
    token: &str,
    task: &str,
    bot: &str,
) -> Result<ResponseSpec, String> {
    let spec = if token.is_empty() {
        RequestSpec::new("")
    } else {
        RequestSpec::new("").with_bearer(token)
    };
    get(base, &status_path(task, bot), &spec).await
}

pub(crate) async fn status_with_key(
    base: &str,
    token: &str,
    task: &str,
    bot: &str,
    key: &str,
) -> Result<ResponseSpec, String> {
    let spec = RequestSpec::new("").with_bearer(token).with_key(key);
    get(base, &status_path(task, bot), &spec).await
}

pub(crate) async fn status_expect(
    base: &str,
    token: &str,
    task: &str,
    bot: &str,
    code: u16,
) -> Result<ResponseSpec, String> {
    let answer = status(base, token, task, bot).await?;
    if answer.status != code {
        return Err(format!("status {}: {}", code, answer.json));
    }
    Ok(answer)
}

pub(crate) async fn records_count(base: &str, token: &str, bot: &str) -> Result<usize, String> {
    let spec = RequestSpec::new("").with_bearer(token);
    let path = format!("/v1/streams/after?stream=bot/{bot}&after=0");
    let answer = get(base, &path, &spec).await?;
    answer
        .json
        .get("records")
        .and_then(Value::as_array)
        .map(Vec::len)
        .ok_or(format!("no records in {}", answer.json))
}
