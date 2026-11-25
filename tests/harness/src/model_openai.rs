use serde_json::json;
use tokio::sync::mpsc;

use crate::model_plan::json_line;
use crate::model_script::Step;

pub const FAKE_CREATED: i64 = 1_700_000_000;
pub const CODE_OPENAI_DONE: &str = "[DONE]";

pub async fn plan_lines(tx: &mpsc::Sender<String>, step: Step, model: &str) {
    let send_done = match step {
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
    if send_done {
        let _sent = tx
            .send(format!("data: {CODE_OPENAI_DONE}\n\n"))
            .await
            .is_ok();
    }
}

async fn send(tx: &mpsc::Sender<String>, value: serde_json::Value) {
    let _sent = tx.send(json_line(&value)).await.is_ok();
}

async fn reply(tx: &mpsc::Sender<String>, model: &str, text: &str) {
    send(tx, role_chunk(model)).await;
    send(tx, content_chunk(model, text)).await;
    send(tx, finish_chunk(model, "stop")).await;
}

async fn think_reply(tx: &mpsc::Sender<String>, model: &str, thinking: &str, text: &str) {
    send(tx, role_chunk(model)).await;
    send(tx, delta_chunk(model, &json!({ "reasoning": thinking }))).await;
    send(tx, content_chunk(model, text)).await;
    send(tx, finish_chunk(model, "stop")).await;
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

fn split_head(text: &str) -> String {
    let bytes = text.as_bytes();
    let Some(mid) = bytes.len().checked_div(2) else {
        return String::new();
    };
    let boundary = bytes.len().min(mid.saturating_add(1));
    let end = bytes.len().min(boundary);
    String::from_utf8_lossy(bytes.get(..end).unwrap_or(bytes)).into_owned()
}

fn role_chunk(model: &str) -> serde_json::Value {
    delta_chunk(model, &json!({ "role": "assistant" }))
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
        "choices": [{ "index": 0, "delta": delta, "finish_reason": null }],
    })
}

fn finish_chunk(model: &str, reason: &str) -> serde_json::Value {
    json!({
        "id": "chatcmpl-fake",
        "object": "chat.completion.chunk",
        "created": FAKE_CREATED,
        "model": model,
        "choices": [{ "index": 0, "delta": {}, "finish_reason": reason }],
    })
}

#[must_use]
pub fn json_body(step: &Step, model: &str) -> String {
    let (message, reason) = completion_message(step);
    let body = json!({
        "id": "chatcmpl-fake",
        "object": "chat.completion",
        "created": FAKE_CREATED,
        "model": model,
        "choices": [{ "index": 0, "message": message, "finish_reason": reason }],
        "usage": { "prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2 },
    });
    body.to_string()
}

fn completion_message(step: &Step) -> (serde_json::Value, &'static str) {
    match step {
        Step::ReplyText { text, .. } => (json!({ "role": "assistant", "content": text }), "stop"),
        Step::ThinkThenReply { thinking, text, .. } => (
            json!({ "role": "assistant", "content": text, "reasoning_content": thinking }),
            "stop",
        ),
        Step::CallTool { name, args, .. } => (
            json!({ "role": "assistant", "content": null, "tool_calls": [{
                "id": "call_fake", "type": "function",
                "function": { "name": name, "arguments": args.to_string() } }] }),
            "tool_calls",
        ),
        Step::NoResponse { .. }
        | Step::CutOffAfter { .. }
        | Step::StallAfter { .. }
        | Step::ReplyStatus { .. } => (json!({ "role": "assistant", "content": "" }), "stop"),
    }
}
