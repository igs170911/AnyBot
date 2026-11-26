use serde_json::json;
use tokio::sync::mpsc;

use crate::model_extra::openai_usage;
use crate::model_plan::json_line;
use crate::model_script::{ToolCallScript, UsageScript};

use super::{FAKE_CREATED, content_chunk, delta_chunk, finish_chunk, role_chunk};

pub(super) async fn send(tx: &mpsc::Sender<String>, value: serde_json::Value) {
    let _sent = tx.send(json_line(&value)).await.is_ok();
}

pub(super) async fn think_content(
    tx: &mpsc::Sender<String>,
    model: &str,
    thinking: &str,
    text: &str,
) {
    send(tx, role_chunk(model)).await;
    send(
        tx,
        delta_chunk(model, &json!({ "reasoning_content": thinking })),
    )
    .await;
    send(tx, content_chunk(model, text)).await;
    send(tx, finish_chunk(model, "stop")).await;
}

pub(super) async fn reply(tx: &mpsc::Sender<String>, model: &str, text: &str) {
    send(tx, role_chunk(model)).await;
    send(tx, content_chunk(model, text)).await;
    send(tx, finish_chunk(model, "stop")).await;
}

pub(super) async fn reply_reason(tx: &mpsc::Sender<String>, model: &str, text: &str, reason: &str) {
    send(tx, role_chunk(model)).await;
    send(tx, content_chunk(model, text)).await;
    send(tx, finish_chunk(model, reason)).await;
}

pub(super) async fn tool_parts(
    tx: &mpsc::Sender<String>,
    model: &str,
    calls: &[ToolCallScript],
    reason: &str,
    cut: bool,
    length_stop: bool,
) -> bool {
    send(tx, role_chunk(model)).await;
    for (index, call) in calls.iter().enumerate() {
        send_parts(tx, model, index, call, cut).await;
    }
    if !cut {
        let finish = if length_stop { "length" } else { reason };
        send(tx, finish_chunk(model, finish)).await;
    }
    !cut
}

async fn send_parts(
    tx: &mpsc::Sender<String>,
    model: &str,
    index: usize,
    call: &ToolCallScript,
    cut: bool,
) {
    let parts = argument_parts(call);
    let shown = if cut {
        parts.len().saturating_sub(1)
    } else {
        parts.len()
    };
    for (part_index, part) in parts.iter().enumerate() {
        if part_index >= shown {
            break;
        }
        let id = if part_index == 0 {
            call.id.clone()
        } else {
            None
        };
        let name = if part_index == 0 {
            Some(call.name.as_str())
        } else {
            None
        };
        send(tx, tool_delta(model, index, id.as_deref(), name, part)).await;
    }
}

fn argument_parts(call: &ToolCallScript) -> Vec<String> {
    if call.argument_parts.is_empty() {
        return vec![call.arguments.clone()];
    }
    call.argument_parts.clone()
}

fn tool_delta(
    model: &str,
    index: usize,
    id: Option<&str>,
    name: Option<&str>,
    arguments: &str,
) -> serde_json::Value {
    let mut function = json!({ "arguments": arguments });
    if let Some(tool_name) = name {
        crate::json_edit::insert(&mut function, "name", json!(tool_name));
    }
    let mut call = json!({ "index": index, "function": function });
    if let Some(call_id) = id {
        crate::json_edit::insert(&mut call, "id", json!(call_id));
        crate::json_edit::insert(&mut call, "type", json!("function"));
    }
    delta_chunk(model, &json!({ "tool_calls": [call] }))
}

pub(super) fn usage_chunk(model: &str, usage: &UsageScript) -> serde_json::Value {
    json!({
        "id": "chatcmpl-fake",
        "object": "chat.completion.chunk",
        "created": FAKE_CREATED,
        "model": model,
        "choices": [],
        "usage": openai_usage(usage),
    })
}

pub(super) async fn tool_call(
    tx: &mpsc::Sender<String>,
    model: &str,
    name: &str,
    args: &serde_json::Value,
) {
    let arguments = args.to_string();
    send(tx, role_chunk(model)).await;
    send(
        tx,
        delta_chunk(
            model,
            &json!({ "tool_calls": [{ "index": 0, "id": "call_fake", "type": "function",
                "function": { "name": name, "arguments": arguments } }] }),
        ),
    )
    .await;
    send(tx, finish_chunk(model, "tool_calls")).await;
}
