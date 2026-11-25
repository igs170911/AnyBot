use serde_json::json;
use tokio::sync::mpsc;

use crate::model_extra::openai_usage;
use crate::model_plan::json_line;
use crate::model_script::{Step, ToolCallScript, UsageScript};

pub const FAKE_CREATED: i64 = 1_700_000_000;
pub const CODE_OPENAI_DONE: &str = "[DONE]";
pub const FAKE_FINGERPRINT: &str = "fake-fingerprint";

#[path = "model_openai_body.rs"]
mod body;

pub use body::json_body;

pub async fn plan_lines(tx: &mpsc::Sender<String>, step: Step, model: &str) {
    let send_done = dispatch(tx, step, model).await;
    if send_done {
        let _sent = tx
            .send(format!("data: {CODE_OPENAI_DONE}\n\n"))
            .await
            .is_ok();
    }
}

async fn dispatch(tx: &mpsc::Sender<String>, step: Step, model: &str) -> bool {
    match step {
        Step::ReplyText { text, .. } => {
            reply(tx, model, &text).await;
            true
        }
        Step::ThinkThenReply { thinking, text, .. }
        | Step::ThinkingBlock { thinking, text, .. } => {
            think_content(tx, model, &thinking, &text).await;
            true
        }
        Step::CallTool { name, args, .. } => {
            tool_call(tx, model, &name, &args).await;
            true
        }
        Step::NoResponse { .. } => silence(tx).await,
        Step::CutOffAfter { text, .. } => cut_off(tx, model, &text).await,
        Step::StallAfter { text, .. } => stall(tx, model, &text).await,
        Step::UsageReply {
            text,
            usage,
            omit_usage,
            ..
        } => usage_reply(tx, model, &text, &usage, omit_usage).await,
        Step::ToolCalls {
            calls,
            finish_reason,
            cut_mid_arguments,
            length_stop,
            ..
        } => {
            tool_parts(
                tx,
                model,
                &calls,
                &finish_reason,
                cut_mid_arguments,
                length_stop,
            )
            .await
        }
        Step::StopReason {
            text, stop_reason, ..
        } => {
            reply_reason(tx, model, &text, &stop_reason).await;
            true
        }
        Step::ThinkThenTool { calls, .. } => {
            tool_parts(tx, model, &calls, "tool_calls", false, false).await;
            true
        }
        Step::ReplyStatus { .. }
        | Step::StreamError { .. }
        | Step::EmptyBody
        | Step::Redirect { .. }
        | Step::ModelList { .. } => false,
    }
}

async fn send(tx: &mpsc::Sender<String>, value: serde_json::Value) {
    let _sent = tx.send(json_line(&value)).await.is_ok();
}

async fn think_content(tx: &mpsc::Sender<String>, model: &str, thinking: &str, text: &str) {
    send(tx, role_chunk(model)).await;
    send(
        tx,
        delta_chunk(model, &json!({ "reasoning_content": thinking })),
    )
    .await;
    send(tx, content_chunk(model, text)).await;
    send(tx, finish_chunk(model, "stop")).await;
}

async fn reply(tx: &mpsc::Sender<String>, model: &str, text: &str) {
    send(tx, role_chunk(model)).await;
    send(tx, content_chunk(model, text)).await;
    send(tx, finish_chunk(model, "stop")).await;
}

async fn usage_reply(
    tx: &mpsc::Sender<String>,
    model: &str,
    text: &str,
    usage: &UsageScript,
    omit: bool,
) -> bool {
    reply(tx, model, text).await;
    if !omit {
        send(tx, usage_chunk(model, usage)).await;
    }
    true
}

async fn reply_reason(tx: &mpsc::Sender<String>, model: &str, text: &str, reason: &str) {
    send(tx, role_chunk(model)).await;
    send(tx, content_chunk(model, text)).await;
    send(tx, finish_chunk(model, reason)).await;
}

async fn tool_parts(
    tx: &mpsc::Sender<String>,
    model: &str,
    calls: &[ToolCallScript],
    reason: &str,
    cut: bool,
    length_stop: bool,
) -> bool {
    send(tx, role_chunk(model)).await;
    for (index, call) in calls.iter().enumerate() {
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
    if !cut {
        let finish = if length_stop { "length" } else { reason };
        send(tx, finish_chunk(model, finish)).await;
    }
    !cut
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

fn usage_chunk(model: &str, usage: &UsageScript) -> serde_json::Value {
    json!({
        "id": "chatcmpl-fake",
        "object": "chat.completion.chunk",
        "created": FAKE_CREATED,
        "model": model,
        "choices": [],
        "usage": openai_usage(usage),
    })
}

async fn tool_call(tx: &mpsc::Sender<String>, model: &str, name: &str, args: &serde_json::Value) {
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

async fn silence(_tx: &mpsc::Sender<String>) -> bool {
    tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
    true
}

async fn cut_off(tx: &mpsc::Sender<String>, model: &str, text: &str) -> bool {
    let head = split_head(text);
    send(tx, role_chunk(model)).await;
    send(tx, content_chunk(model, &head)).await;
    false
}

async fn stall(tx: &mpsc::Sender<String>, model: &str, text: &str) -> bool {
    let head = split_head(text);
    send(tx, role_chunk(model)).await;
    send(tx, content_chunk(model, &head)).await;
    tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
    let _sent = tx.is_closed();
    true
}

pub(crate) fn split_head(text: &str) -> String {
    let bytes = text.as_bytes();
    let Some(mid) = bytes.len().checked_div(2) else {
        return String::new();
    };
    let boundary = bytes.len().min(mid.saturating_add(1));
    let end = bytes.len().min(boundary);
    String::from_utf8_lossy(bytes.get(..end).unwrap_or(bytes)).into_owned()
}

fn content_chunk(model: &str, text: &str) -> serde_json::Value {
    delta_chunk(model, &json!({ "content": text }))
}

fn delta_chunk(model: &str, delta: &serde_json::Value) -> serde_json::Value {
    json!({
        "id": "chatcmpl-fake",
        "object": "chat.completion.chunk",
        "created": FAKE_CREATED,
        "model": model,
        "choices": [{
            "index": 0,
            "delta": delta,
            "logprobs": null,
            "finish_reason": null,
        }],
    })
}

fn role_chunk(model: &str) -> serde_json::Value {
    delta_chunk(model, &json!({ "role": "assistant", "content": "" }))
}

fn finish_chunk(model: &str, reason: &str) -> serde_json::Value {
    let mut chunk = delta_chunk(model, &json!({}));
    if let Some(choice) = chunk.get_mut("choices").and_then(|c| c.get_mut(0)) {
        choice["finish_reason"] = json!(reason);
    }
    chunk
}
