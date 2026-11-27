use serde_json::json;

use anybot_store_server::{Pool, streams};

use crate::app::AppState;
use crate::req_write::{CallerDenied, internal_sqlx, internal_text};

pub const KIND_COMPUTERS_CHANGED: &str = "computers_changed";

pub(crate) async fn announce(
    state: &AppState,
    pool: &Pool,
    account: &str,
    change: &str,
    device_id: &str,
) -> Result<(), CallerDenied> {
    let now = i64::try_from(state.clock.now_ms().millis()).map_err(internal_text)?;
    let stream = format!("account/{account}");
    streams::ensure_stream(pool, &stream, "account", account, None, now)
        .await
        .map_err(internal_sqlx)?;
    let content = json!({"change": change, "device_id": device_id}).to_string();
    let record_id = uuid::Uuid::now_v7().to_string();
    let spec = streams::InsertSpec {
        device_id: &record_id,
        kind: KIND_COMPUTERS_CHANGED,
        content: &content,
        created_at_ms: now,
        offline_sent: false,
        task_id: None,
    };
    let seq = streams::insert_record(pool, &stream, &spec)
        .await
        .map_err(internal_sqlx)?;
    let note =
        anybot_bus::Notification::with_seq(&anybot_bus::stream_note(&stream), &*state.clock, seq);
    state.bus.publish(note);
    Ok(())
}
