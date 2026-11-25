use serde_json::json;
use tokio::sync::mpsc;

use super::{FAKE_MESSAGE_ID, send};
use crate::model_extra::anthropic_usage;
use crate::model_script::{ToolCallScript, UsageScript};

pub(super) async fn anthropic_tools(
    tx: &mpsc::Sender<String>,
    model: &str,
    calls: &[ToolCallScript],
    reason: &str,
    cut: bool,
    length_stop: bool,
) -> bool {
    send(tx, message_start(model)).await;
    for (index, call) in calls.iter().enumerate() {
        let id = call.id.clone().unwrap_or_else(|| format!("toolu_{index}"));
        let idx = u8::try_from(index).unwrap_or(u8::MAX);
        send(
            tx,
            json!({ "type": "content_block_start", "index": idx,
                "content_block": { "type": "tool_use", "id": id, "name": call.name, "input": {} } }),
        )
        .await;
        let parts = if call.argument_parts.is_empty() {
            vec![call.arguments.clone()]
        } else {
            call.argument_parts.clone()
        };
        let shown = if cut {
            parts.len().saturating_sub(1)
        } else {
            parts.len()
        };
        for (part_index, part) in parts.iter().enumerate() {
            if part_index >= shown {
                break;
            }
            send(
                tx,
                json!({ "type": "content_block_delta", "index": idx,
                    "delta": { "type": "input_json_delta", "partial_json": part } }),
            )
            .await;
        }
        if !cut {
            send(tx, content_block_stop(idx)).await;
        }
    }
    if !cut {
        let stop = if length_stop { "max_tokens" } else { reason };
        send(tx, message_delta(stop)).await;
    }
    !cut
}

pub(super) async fn think_then_tools(
    tx: &mpsc::Sender<String>,
    model: &str,
    thinking: &str,
    signature: &str,
    calls: &[ToolCallScript],
) {
    send(tx, message_start(model)).await;
    send(
        tx,
        json!({ "type": "content_block_start", "index": 0,
            "content_block": { "type": "thinking", "thinking": "", "signature": "" } }),
    )
    .await;
    send(
        tx,
        json!({ "type": "content_block_delta", "index": 0,
            "delta": { "type": "thinking_delta", "thinking": thinking } }),
    )
    .await;
    send(
        tx,
        json!({ "type": "content_block_delta", "index": 0,
            "delta": { "type": "signature_delta", "signature": signature } }),
    )
    .await;
    send(tx, content_block_stop(0)).await;
    for (index, call) in calls.iter().enumerate() {
        let idx = u8::try_from(index.saturating_add(1)).unwrap_or(u8::MAX);
        let id = call.id.clone().unwrap_or_else(|| format!("toolu_{index}"));
        send(
            tx,
            json!({ "type": "content_block_start", "index": idx,
                "content_block": { "type": "tool_use", "id": id, "name": call.name, "input": {} } }),
        )
        .await;
        send(
            tx,
            json!({ "type": "content_block_delta", "index": idx,
                "delta": { "type": "input_json_delta", "partial_json": call.arguments } }),
        )
        .await;
        send(tx, content_block_stop(idx)).await;
    }
    send(tx, message_delta("tool_use")).await;
}

pub(super) fn message_delta_usage(stop_reason: &str, usage: &UsageScript) -> serde_json::Value {
    json!({ "type": "message_delta",
        "delta": { "stop_reason": stop_reason, "stop_details": null },
        "usage": anthropic_usage(usage) })
}

pub(super) async fn silence(_tx: &mpsc::Sender<String>) -> bool {
    tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
    true
}

pub(super) async fn cut_off(tx: &mpsc::Sender<String>, model: &str, text: &str) -> bool {
    send(tx, message_start(model)).await;
    send(tx, text_block_start()).await;
    send(tx, text_delta(&split_head(text))).await;
    false
}

pub(super) async fn stall(tx: &mpsc::Sender<String>, model: &str, text: &str) -> bool {
    send(tx, message_start(model)).await;
    send(tx, text_block_start()).await;
    send(tx, text_delta(&split_head(text))).await;
    tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
    let _closed = tx.is_closed();
    true
}

fn split_head(text: &str) -> String {
    let bytes = text.as_bytes();
    let Some(mid) = bytes.len().checked_div(2) else {
        return String::new();
    };
    let boundary = bytes.len().min(mid.saturating_add(1));
    let end = bytes.len().min(boundary);
    String::from_utf8_lossy(bytes.get(..end).unwrap_or(bytes)).into_owned()
}

pub(super) fn message_start(model: &str) -> serde_json::Value {
    json!({
        "type": "message_start",
        "message": {
            "id": FAKE_MESSAGE_ID,
            "type": "message",
            "role": "assistant",
            "model": model,
            "content": [],
            "usage": { "input_tokens": 1, "output_tokens": 1 },
        },
    })
}

pub(super) fn text_block_start() -> serde_json::Value {
    text_block_start_index(0)
}

pub(super) fn text_block_start_index(index: u8) -> serde_json::Value {
    json!({ "type": "content_block_start", "index": index,
        "content_block": { "type": "text", "text": "" } })
}

pub(super) fn text_delta(text: &str) -> serde_json::Value {
    text_delta_at(0, text)
}

pub(super) fn text_delta_at(index: u8, text: &str) -> serde_json::Value {
    json!({ "type": "content_block_delta", "index": index,
        "delta": { "type": "text_delta", "text": text } })
}

pub(super) fn content_block_stop(index: u8) -> serde_json::Value {
    json!({ "type": "content_block_stop", "index": index })
}

pub(super) fn message_delta(stop_reason: &str) -> serde_json::Value {
    json!({ "type": "message_delta",
        "delta": { "stop_reason": stop_reason, "stop_details": null },
        "usage": { "output_tokens": 1 } })
}
