use serde_json::json;
use tokio::sync::mpsc;

use crate::model_plan::json_line;
use crate::model_script::Step;

pub const FAKE_MESSAGE_ID: &str = "msg_fake_0123456789";

#[path = "model_anthropic_body.rs"]
mod body;

pub use body::json_body;

#[path = "model_anthropic_stream.rs"]
mod stream;

use stream::anthropic_tools;

pub async fn plan_lines(tx: &mpsc::Sender<String>, step: Step, model: &str) {
    let send_stop = dispatch(tx, step, model).await;
    if send_stop {
        send(tx, json!({ "type": "message_stop" })).await;
    }
}

async fn dispatch(tx: &mpsc::Sender<String>, step: Step, model: &str) -> bool {
    match step {
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
        Step::NoResponse { .. } => stream::silence(tx).await,
        Step::CutOffAfter { text, .. } => stream::cut_off(tx, model, &text).await,
        Step::StallAfter { text, .. } => stream::stall(tx, model, &text).await,
        Step::HoldThenFinish { text, .. } => stream::hold_then_finish(tx, model, &text).await,
        Step::UsageReply { text, usage, .. } => {
            reply(tx, model, &text).await;
            send(tx, stream::message_delta_usage("end_turn", &usage)).await;
            true
        }
        Step::ThinkingBlock {
            thinking,
            signature,
            text,
            redacted,
            ..
        } => {
            signed_think(tx, model, &thinking, &signature, &text, redacted).await;
            true
        }
        Step::StopReason {
            text, stop_reason, ..
        } => {
            reply(tx, model, &text).await;
            send(tx, stream::message_delta(stop_reason.as_str())).await;
            true
        }
        other @ (Step::ToolCalls { .. }
        | Step::ThinkThenTool { .. }
        | Step::ReplyStatus { .. }
        | Step::StreamError { .. }
        | Step::EmptyBody
        | Step::Redirect { .. }
        | Step::ModelList { .. }
        | Step::DelayHeaders { .. }
        | Step::AcceptThenDelay { .. }) => dispatch_rest(tx, other, model).await,
    }
}

async fn dispatch_rest(tx: &mpsc::Sender<String>, step: Step, model: &str) -> bool {
    match step {
        Step::ToolCalls {
            calls,
            finish_reason,
            cut_mid_arguments,
            length_stop,
            ..
        } => {
            anthropic_tools(
                tx,
                model,
                &calls,
                &finish_reason,
                cut_mid_arguments,
                length_stop,
            )
            .await
        }
        Step::ThinkThenTool {
            thinking,
            signature,
            calls,
            ..
        } => {
            stream::think_then_tools(tx, model, &thinking, &signature, &calls).await;
            true
        }
        Step::ReplyText { .. }
        | Step::ThinkThenReply { .. }
        | Step::CallTool { .. }
        | Step::NoResponse { .. }
        | Step::CutOffAfter { .. }
        | Step::StallAfter { .. }
        | Step::HoldThenFinish { .. }
        | Step::UsageReply { .. }
        | Step::ThinkingBlock { .. }
        | Step::StopReason { .. }
        | Step::ReplyStatus { .. }
        | Step::StreamError { .. }
        | Step::EmptyBody
        | Step::Redirect { .. }
        | Step::ModelList { .. }
        | Step::DelayHeaders { .. }
        | Step::AcceptThenDelay { .. } => false,
    }
}

pub(crate) async fn send(tx: &mpsc::Sender<String>, value: serde_json::Value) {
    let _sent = tx.send(json_line(&value)).await.is_ok();
}

async fn reply(tx: &mpsc::Sender<String>, model: &str, text: &str) {
    send(tx, stream::message_start(model)).await;
    send(tx, stream::text_block_start()).await;
    send(tx, stream::text_delta(text)).await;
    send(tx, stream::content_block_stop(0)).await;
    send(tx, stream::message_delta("end_turn")).await;
}

async fn think_reply(tx: &mpsc::Sender<String>, model: &str, thinking: &str, text: &str) {
    send(tx, stream::message_start(model)).await;
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
    send(tx, stream::content_block_stop(0)).await;
    send(tx, stream::text_block_start_index(1)).await;
    send(tx, stream::text_delta_at(1, text)).await;
    send(tx, stream::content_block_stop(1)).await;
    send(tx, stream::message_delta("end_turn")).await;
}

async fn tool_call(tx: &mpsc::Sender<String>, model: &str, name: &str, args: &serde_json::Value) {
    send(tx, stream::message_start(model)).await;
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
    send(tx, stream::content_block_stop(0)).await;
    send(tx, stream::message_delta("tool_use")).await;
}

async fn signed_think(
    tx: &mpsc::Sender<String>,
    model: &str,
    thinking: &str,
    signature: &str,
    text: &str,
    redacted: bool,
) {
    send(tx, stream::message_start(model)).await;
    if redacted {
        send(
            tx,
            json!({ "type": "content_block_start", "index": 0,
                "content_block": { "type": "redacted_thinking", "data": signature } }),
        )
        .await;
        send(tx, stream::content_block_stop(0)).await;
    } else {
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
        send(tx, stream::content_block_stop(0)).await;
    }
    send(tx, stream::text_block_start_index(1)).await;
    send(tx, stream::text_delta_at(1, text)).await;
    send(tx, stream::content_block_stop(1)).await;
    send(tx, stream::message_delta("end_turn")).await;
}
