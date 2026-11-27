use serde_json::{Value, json};

use anybot_app_test::clockwalk::ClockWalker;
use anybot_app_test::http::{RequestSpec, ResponseSpec, post};
use anybot_app_test::serve::{Served, boot_pair};
use sqlx::Row;

pub(crate) const BOT: &str = "bot-r1";
pub(crate) const TASK: &str = "task-r1";

pub(crate) struct Device {
    pub(crate) token: String,
    pub(crate) id: String,
}

pub(crate) async fn login_device(served: &Served, key: &str) -> Result<Device, String> {
    let body = json!({
        "account": "alice",
        "password": "alicepassword",
        "label": key,
        "credential": "bearer",
    })
    .to_string();
    let spec = RequestSpec::new(&body).with_key(key);
    let mut walker = ClockWalker::start(&served.clock);
    let answer = post(&served.base_url(), "/v1/account/login", &spec).await;
    walker.stop().await?;
    let answer = answer?;
    Ok(Device {
        token: answer.bearer()?,
        id: answer.field_str("device_id")?,
    })
}

pub(crate) async fn send_stream(
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

pub(crate) async fn start(served: &Served) -> Result<Device, String> {
    boot_pair(served).await?;
    let device = login_device(served, "k-referee-device-a").await?;
    let create = json!({
        "device_id": "dev-create-bot",
        "kind": "bots_create",
        "content": {"bot_id": BOT, "name": "helper"},
        "created_at_ms": 1_700_000_000_000_i64,
        "offline_sent": false,
    });
    let landed = send_stream(
        served,
        &device.token,
        "k-referee-bot",
        &json!("account"),
        &[create],
    )
    .await?;
    if landed.status != 200 {
        return Err(format!("bot not created: {}", landed.json));
    }
    Ok(device)
}

pub(crate) async fn task_record(
    served: &Served,
    token: &str,
    key: &str,
    kind: &str,
    content: &Value,
) -> Result<(), String> {
    let item = json!({
        "device_id": format!("dev-{key}"),
        "kind": kind,
        "content": content,
        "created_at_ms": 1_700_000_000_100_i64,
        "offline_sent": false,
        "task_id": TASK,
    });
    let landed = send_stream(served, token, key, &json!({"bot": {"id": BOT}}), &[item]).await?;
    let tag = landed
        .json
        .get("results")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .and_then(|row| row.get("result"))
        .and_then(Value::as_str);
    if tag == Some("accepted") {
        return Ok(());
    }
    Err(format!("task record refused: {}", landed.json))
}

pub(crate) async fn show_card(
    served: &Served,
    token: &str,
    key: &str,
    card: &str,
) -> Result<(), String> {
    task_record(
        served,
        token,
        key,
        "card_shown",
        &json!({"action": "shell", "card": card}),
    )
    .await
}

pub(crate) async fn referee(
    served: &Served,
    token: &str,
    key: &str,
    body: &Value,
) -> Result<ResponseSpec, String> {
    let spec = RequestSpec::new(&body.to_string())
        .with_key(key)
        .with_bearer(token);
    post(&served.base_url(), "/v1/streams/referee", &spec).await
}

pub(crate) fn card_answer(card: Option<&str>, answer: &str) -> Value {
    let mut body = json!({
        "stream": {"bot": {"id": BOT}},
        "task_id": TASK,
        "action": "card_answer",
        "answer": answer,
    });
    if let (Some(card), Some(map)) = (card, body.as_object_mut()) {
        map.insert("card_id".to_owned(), json!(card));
    }
    body
}

pub(crate) fn plain_action(action: &str) -> Value {
    json!({"stream": {"bot": {"id": BOT}}, "task_id": TASK, "action": action})
}

pub(crate) async fn count_of(served: &Served, sql: &'static str) -> Result<i64, String> {
    let row = sqlx::query(sql)
        .fetch_one(&served.pool)
        .await
        .map_err(|e| e.to_string())?;
    row.try_get::<i64, _>("n").map_err(|e| e.to_string())
}

pub(crate) async fn answered_by(served: &Served, card: &str) -> Result<String, String> {
    let row = sqlx::query("SELECT device_id FROM card_answers WHERE card_id = $1")
        .bind(card)
        .fetch_one(&served.pool)
        .await
        .map_err(|e| e.to_string())?;
    row.try_get::<String, _>("device_id")
        .map_err(|e| e.to_string())
}
