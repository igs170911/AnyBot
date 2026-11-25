use serde_json::json;
use tokio::sync::mpsc;

use crate::model_plan::json_line;
use crate::model_script::Step;

pub const FAKE_MESSAGE_ID: &str = "msg_fake_0123456789";

pub async fn plan_lines(tx: &mpsc::Sender<String>, step: Step, model: &str) {
    let send_stop = match step {
        Step::ReplyText { text, .. } => {
            reply(tx, model, &text).await;
            true
        }
        Step::ThinkThenReply { thinking, text, .. } => {
            think_reply(tx, model, &thinking, &text).await;
            true
        }
        Step::CallTool { name, args, .. } => {
            tool_call(tx, model, &name, &args).await;
            true
        }
        Step::NoResponse { .. } => silence(tx).await,
        Step::CutOffAfter { text, .. } => cut_off(tx, model, &text).await,
        Step::StallAfter { text, .. } => stall(tx, model, &text).await,
        Step::ReplyStatus { .. } => false,
    };
    if send_stop {
        send(tx, json!({ "type": "message_stop" })).await;
    }
}

async fn send(tx: &mpsc::Sender<String>, value: serde_json::Value) {
    let _sent = tx.send(json_line(&value)).await.is_ok();
}

async fn reply(tx: &mpsc::Sender<String>, model: &str, text: &str) {
    send(tx, message_start(model)).await;
    send(tx, text_block_start()).await;
    send(tx, text_delta(text)).await;
    send(tx, content_block_stop(0)).await;
    send(tx, message_delta("end_turn")).await;
}

async fn think_reply(tx: &mpsc::Sender<String>, model: &str, thinking: &str, text: &str) {
    send(tx, message_start(model)).await;
    send(
        tx,
        json!({ "type": "content_block_start", "index": 0,
        "content_block": { "type": "thinking", "thinking": "" } }),
    )
    .await;
    send(
        tx,
        json!({ "type": "content_block_delta", "index": 0,
        "delta": { "type": "thinking_delta", "thinking": thinking } }),
    )
    .await;
    send(tx, content_block_stop(0)).await;
    send(tx, text_block_start_index(1)).await;
    send(tx, text_delta_at(1, text)).await;
    send(tx, content_block_stop(1)).await;
    send(tx, message_delta("end_turn")).await;
}

async fn tool_call(tx: &mpsc::Sender<String>, model: &str, name: &str, args: &serde_json::Value) {
    send(tx, message_start(model)).await;
    send(
        tx,
        json!({ "type": "content_block_start", "index": 0,
            "content_block": { "type": "tool_use", "id": "toolu_fake", "name": name, "input": {} } }),
    )
    .await;
    send(
        tx,
        json!({ "type": "content_block_delta", "index": 0,
            "delta": { "type": "input_json_delta", "partial_json": args.to_string() } }),
    )
    .await;
    send(tx, content_block_stop(0)).await;
    send(tx, message_delta("tool_use")).await;
}

async fn silence(_tx: &mpsc::Sender<String>) -> bool {
    tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
    true
}

async fn cut_off(tx: &mpsc::Sender<String>, model: &str, text: &str) -> bool {
    send(tx, message_start(model)).await;
    send(tx, text_block_start()).await;
    send(tx, text_delta(&split_head(text))).await;
    false
}

async fn stall(tx: &mpsc::Sender<String>, model: &str, text: &str) -> bool {
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

fn message_start(model: &str) -> serde_json::Value {
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

fn text_block_start() -> serde_json::Value {
    text_block_start_index(0)
}

fn text_block_start_index(index: u8) -> serde_json::Value {
    json!({ "type": "content_block_start", "index": index,
        "content_block": { "type": "text", "text": "" } })
}

fn text_delta(text: &str) -> serde_json::Value {
    text_delta_at(0, text)
}

fn text_delta_at(index: u8, text: &str) -> serde_json::Value {
    json!({ "type": "content_block_delta", "index": index,
        "delta": { "type": "text_delta", "text": text } })
}

fn content_block_stop(index: u8) -> serde_json::Value {
    json!({ "type": "content_block_stop", "index": index })
}

fn message_delta(stop_reason: &str) -> serde_json::Value {
    json!({ "type": "message_delta",
        "delta": { "stop_reason": stop_reason, "stop_details": null },
        "usage": { "output_tokens": 1 } })
}

#[must_use]
pub fn json_body(step: &Step, model: &str) -> String {
    let (blocks, reason) = content_blocks(step);
    let body = json!({
        "id": FAKE_MESSAGE_ID,
        "type": "message",
        "role": "assistant",
        "model": model,
        "content": blocks,
        "stop_reason": reason,
        "stop_details": null,
        "usage": { "input_tokens": 1, "output_tokens": 1 },
    });
    body.to_string()
}

fn content_blocks(step: &Step) -> (Vec<serde_json::Value>, &'static str) {
    match step {
        Step::ReplyText { text, .. } => (vec![text_block(text)], "end_turn"),
        Step::ThinkThenReply { thinking, text, .. } => (
            vec![
                json!({ "type": "thinking", "thinking": thinking }),
                text_block(text),
            ],
            "end_turn",
        ),
        Step::CallTool { name, args, .. } => (
            vec![json!({ "type": "tool_use", "id": "toolu_fake", "name": name, "input": args })],
            "tool_use",
        ),
        Step::NoResponse { .. }
        | Step::CutOffAfter { .. }
        | Step::StallAfter { .. }
        | Step::ReplyStatus { .. } => (Vec::new(), "end_turn"),
    }
}

fn text_block(text: &str) -> serde_json::Value {
    json!({ "type": "text", "text": text })
}
