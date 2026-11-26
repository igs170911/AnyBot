use serde_json::json;
use tokio::sync::mpsc;

use crate::model_script::{Step, ToolCallScript, UsageScript};

pub const FAKE_CREATED: i64 = 1_700_000_000;
pub const CODE_OPENAI_DONE: &str = "[DONE]";
pub const FAKE_FINGERPRINT: &str = "fake-fingerprint";

#[path = "model_openai_body.rs"]
mod body;
#[path = "model_openai_stream.rs"]
mod stream;

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
        Step::ReplyText { text, .. } => reply_done(tx, model, &text).await,
        Step::ThinkThenReply { thinking, text, .. }
        | Step::ThinkingBlock { thinking, text, .. } => {
            think_done(tx, model, &thinking, &text).await
        }
        Step::CallTool { name, args, .. } => call_done(tx, model, &name, &args).await,
        Step::NoResponse { .. } => silence(tx).await,
        Step::CutOffAfter { text, .. } => cut_off(tx, model, &text).await,
        Step::StallAfter { text, .. } => stall(tx, model, &text).await,
        Step::HoldThenFinish { text, .. } => hold_then_finish(tx, model, &text).await,
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
            tool_calls(
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
            stream::reply_reason(tx, model, &text, &stop_reason).await;
            true
        }
        Step::ThinkThenTool { calls, .. } => {
            stream::tool_parts(tx, model, &calls, "tool_calls", false, false).await;
            true
        }
        Step::ReplyStatus { .. }
        | Step::StreamError { .. }
        | Step::EmptyBody
        | Step::Redirect { .. }
        | Step::ModelList { .. }
        | Step::DelayHeaders { .. }
        | Step::AcceptThenDelay { .. } => false,
    }
}

async fn reply_done(tx: &mpsc::Sender<String>, model: &str, text: &str) -> bool {
    stream::reply(tx, model, text).await;
    true
}

async fn think_done(tx: &mpsc::Sender<String>, model: &str, thinking: &str, text: &str) -> bool {
    stream::think_content(tx, model, thinking, text).await;
    true
}

async fn call_done(
    tx: &mpsc::Sender<String>,
    model: &str,
    name: &str,
    args: &serde_json::Value,
) -> bool {
    stream::tool_call(tx, model, name, args).await;
    true
}

async fn tool_calls(
    tx: &mpsc::Sender<String>,
    model: &str,
    calls: &[ToolCallScript],
    finish_reason: &str,
    cut_mid_arguments: bool,
    length_stop: bool,
) -> bool {
    stream::tool_parts(
        tx,
        model,
        calls,
        finish_reason,
        cut_mid_arguments,
        length_stop,
    )
    .await
}

async fn usage_reply(
    tx: &mpsc::Sender<String>,
    model: &str,
    text: &str,
    usage: &UsageScript,
    omit: bool,
) -> bool {
    stream::reply(tx, model, text).await;
    if !omit {
        stream::send(tx, stream::usage_chunk(model, usage)).await;
    }
    true
}

async fn silence(_tx: &mpsc::Sender<String>) -> bool {
    tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
    true
}

async fn cut_off(tx: &mpsc::Sender<String>, model: &str, text: &str) -> bool {
    let head = split_head(text);
    stream::send(tx, role_chunk(model)).await;
    stream::send(tx, content_chunk(model, &head)).await;
    false
}

async fn hold_then_finish(tx: &mpsc::Sender<String>, model: &str, text: &str) -> bool {
    let head = split_head(text);
    stream::send(tx, role_chunk(model)).await;
    stream::send(tx, content_chunk(model, &head)).await;
    crate::model_hold::wait_release().await;
    stream::send(tx, content_chunk(model, text)).await;
    stream::send(tx, finish_chunk(model, "stop")).await;
    true
}

async fn stall(tx: &mpsc::Sender<String>, model: &str, text: &str) -> bool {
    let head = split_head(text);
    stream::send(tx, role_chunk(model)).await;
    stream::send(tx, content_chunk(model, &head)).await;
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

pub(crate) fn content_chunk(model: &str, text: &str) -> serde_json::Value {
    delta_chunk(model, &json!({ "content": text }))
}

pub(super) fn delta_chunk(model: &str, delta: &serde_json::Value) -> serde_json::Value {
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

pub(super) fn role_chunk(model: &str) -> serde_json::Value {
    delta_chunk(model, &json!({ "role": "assistant", "content": "" }))
}

pub(super) fn finish_chunk(model: &str, reason: &str) -> serde_json::Value {
    let mut chunk = delta_chunk(model, &json!({}));
    if let Some(choice) = chunk.get_mut("choices").and_then(|c| c.get_mut(0)) {
        choice["finish_reason"] = json!(reason);
    }
    chunk
}
