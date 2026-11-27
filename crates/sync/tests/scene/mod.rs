pub(crate) mod tap;

use anybot_app_test::http::{RequestSpec, post};
use anybot_store_client::LocalRecord;
use anybot_sync::{StreamTarget, SyncClient};
use serde_json::{Value, json};

use crate::common::{Rig, ServerRow, message, wait_until};

pub(crate) const BOT: &str = "bot-1";
pub(crate) const BOT_STREAM_ID: &str = "bot/bot-1";

pub(crate) fn bot() -> StreamTarget {
    StreamTarget::Bot(BOT.to_owned())
}

pub(crate) fn device_id(number: u64) -> String {
    format!("0190aaaa-bbbb-7ccc-8ddd-{number:012}")
}

fn item(id: &str, kind: &str, content: &Value) -> Value {
    json!({
        "device_id": id,
        "kind": kind,
        "content": content,
        "created_at_ms": 1_700_000_000_000_i64,
        "offline_sent": false,
    })
}

async fn append(rig: &Rig, stream: Value, items: Vec<Value>, key: &str) -> Result<(), String> {
    let body = json!({"stream": stream, "items": items}).to_string();
    let spec = RequestSpec::new(&body)
        .with_key(key)
        .with_bearer(&rig.bearer);
    let answer = post(&rig.served.base_url(), "/v1/streams/append", &spec).await?;
    if answer.status != 200 {
        return Err(format!("append {key}: {} {}", answer.status, answer.json));
    }
    Ok(())
}

pub(crate) async fn create_bot(rig: &Rig) -> Result<(), String> {
    let create = item(
        &device_id(1),
        "bots_create",
        &json!({"bot_id": BOT, "name": "Bee"}),
    );
    append(rig, json!("account"), vec![create], "scene-create-bot").await
}

pub(crate) async fn phone_sends(
    rig: &Rig,
    first: u64,
    count: u64,
    tag: &str,
) -> Result<(), String> {
    let mut number = first;
    let end = first.saturating_add(count);
    while number < end {
        let batch_end = number.saturating_add(1000).min(end);
        let items = (number..batch_end)
            .map(|n| item(&device_id(n), "message", &message(&format!("p{n}"))))
            .collect();
        let key = format!("scene-{tag}-{number}");
        append(rig, json!({"bot": {"id": BOT}}), items, &key).await?;
        number = batch_end;
    }
    Ok(())
}

pub(crate) async fn local_bot_records(client: &SyncClient) -> Result<Vec<LocalRecord>, String> {
    client.records(&bot()).await.map_err(|e| e.to_string())
}

pub(crate) async fn local_has(client: &SyncClient, count: usize) -> Result<bool, String> {
    Ok(local_bot_records(client).await?.len() == count)
}

pub(crate) async fn wait_local(client: &SyncClient, count: usize) -> Result<(), String> {
    wait_until("the local replica has the records", || async {
        local_has(client, count).await
    })
    .await
}

pub(crate) fn same_as_server(local: &[LocalRecord], server: &[ServerRow]) -> bool {
    local.len() == server.len()
        && local.iter().zip(server).all(|(mine, theirs)| {
            mine.seq == theirs.seq
                && mine.record_id == theirs.device_id
                && mine.kind == theirs.kind
                && mine.content == theirs.content
        })
}

pub(crate) fn contiguous(rows: &[ServerRow]) -> bool {
    rows.iter()
        .zip(1_i64..)
        .all(|(row, expected)| row.seq == expected)
}
