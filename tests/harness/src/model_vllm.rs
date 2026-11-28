use serde_json::json;
use tokio::sync::mpsc;

use crate::model_openai::CODE_OPENAI_DONE;
use crate::model_script::Step;

pub const VLLM_TOOL_FINISH: &str = "stop";
use crate::model_vllm_body as body;
pub use crate::model_vllm_body::json_body;
use crate::model_vllm_chunks as chunks;
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
            think_reply(tx, model, "", &text).await;
            true
        }
        Step::ThinkThenReply { thinking, text, .. }
        | Step::ThinkingBlock { thinking, text, .. } => {
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
        Step::HoldThenFinish { text, .. } => hold_then_finish(tx, model, &text).await,
        Step::UsageReply {
            text,
            usage,
            omit_usage,
            ..
        } => {
            think_reply(tx, model, "", &text).await;
            if !omit_usage {
                send(tx, usage_value_chunk(model, &usage)).await;
            }
            true
        }
        other @ (Step::ToolCalls { .. }
        | Step::StopReason { .. }
        | Step::ThinkThenTool { .. }
        | Step::ToolCallsWithUsage { .. }
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
            vllm_tools(
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
            think_reply(tx, model, "", &text).await;
            send(tx, finish_chunk(model, &stop_reason)).await;
            true
        }
        Step::ThinkThenTool {
            thinking, calls, ..
        } => {
            think_tools(tx, model, &thinking, &calls).await;
            true
        }
        Step::ToolCallsWithUsage { calls, usage, .. } => {
            let ok = vllm_tools(tx, model, &calls, VLLM_TOOL_FINISH, false, false).await;
            send(tx, usage_value_chunk(model, &usage)).await;
            ok
        }
        Step::ReplyText { .. }
        | Step::ThinkThenReply { .. }
        | Step::ThinkingBlock { .. }
        | Step::CallTool { .. }
        | Step::NoResponse { .. }
        | Step::CutOffAfter { .. }
        | Step::StallAfter { .. }
        | Step::HoldThenFinish { .. }
        | Step::UsageReply { .. }
        | Step::ReplyStatus { .. }
        | Step::StreamError { .. }
        | Step::EmptyBody
        | Step::Redirect { .. }
        | Step::ModelList { .. }
        | Step::DelayHeaders { .. }
        | Step::AcceptThenDelay { .. } => false,
    }
}
async fn send(tx: &mpsc::Sender<String>, value: serde_json::Value) {
    chunks::send(tx, value).await;
}
async fn think_reply(tx: &mpsc::Sender<String>, model: &str, thinking: &str, text: &str) {
    send(tx, role_chunk(model)).await;
    send(tx, delta_chunk(model, &json!({ "reasoning": thinking }))).await;
    send(tx, content_chunk(model, text)).await;
    send(tx, finish_chunk(model, VLLM_TOOL_FINISH)).await;
    send(tx, body::usage_chunk(model)).await;
}

pub(crate) use chunks::think_tools;

pub(crate) fn vllm_think_tools(
    thinking: &str,
    calls: &[crate::model_script::ToolCallScript],
) -> serde_json::Value {
    let mut message = body::vllm_tools_message(calls);
    if let Some(map) = message.as_object_mut() {
        map.insert("reasoning".to_owned(), json!(thinking));
    }
    message
}

async fn tool_call(tx: &mpsc::Sender<String>, model: &str, name: &str, args: &serde_json::Value) {
    let arguments = args.to_string();
    send(tx, role_chunk(model)).await;
    send(tx, delta_chunk(model, &json!({ "reasoning": "" }))).await;
    send(
        tx,
        delta_chunk(
            model,
            &json!({ "tool_calls": [{ "index": 0, "id": "call_fake", "type": "function",
                "function": { "name": name, "arguments": arguments } }] }),
        ),
    )
    .await;
    send(tx, finish_chunk(model, VLLM_TOOL_FINISH)).await;
    send(tx, body::usage_chunk(model)).await;
}

async fn silence(_tx: &mpsc::Sender<String>) -> bool {
    tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
    true
}

async fn cut_off(tx: &mpsc::Sender<String>, model: &str, text: &str) -> bool {
    let head = crate::model_openai::split_head(text);
    send(tx, role_chunk(model)).await;
    send(tx, content_chunk(model, &head)).await;
    false
}

async fn hold_then_finish(tx: &mpsc::Sender<String>, model: &str, text: &str) -> bool {
    let head = crate::model_openai::split_head(text);
    send(tx, role_chunk(model)).await;
    send(tx, content_chunk(model, &head)).await;
    crate::model_hold::wait_release().await;
    send(tx, content_chunk(model, text)).await;
    send(tx, finish_chunk(model, "stop")).await;
    true
}

async fn stall(tx: &mpsc::Sender<String>, model: &str, text: &str) -> bool {
    let head = crate::model_openai::split_head(text);
    send(tx, role_chunk(model)).await;
    send(tx, content_chunk(model, &head)).await;
    tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
    let _sent = tx.is_closed();
    true
}

async fn vllm_tools(
    tx: &mpsc::Sender<String>,
    model: &str,
    calls: &[crate::model_script::ToolCallScript],
    reason: &str,
    cut: bool,
    length_stop: bool,
) -> bool {
    send(tx, role_chunk(model)).await;
    for (index, call) in calls.iter().enumerate() {
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
            send(tx, vllm_tool_delta(model, index, id.as_deref(), name, part)).await;
        }
    }
    if !cut {
        let finish = if length_stop { "length" } else { reason };
        send(tx, finish_chunk(model, finish)).await;
    }
    !cut
}

fn vllm_tool_delta(
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

fn usage_value_chunk(model: &str, usage: &crate::model_script::UsageScript) -> serde_json::Value {
    chunks::usage_value_chunk(model, usage)
}

fn content_chunk(model: &str, text: &str) -> serde_json::Value {
    chunks::content_chunk(model, text)
}

fn delta_chunk(model: &str, delta: &serde_json::Value) -> serde_json::Value {
    chunks::delta_chunk(model, delta)
}

fn role_chunk(model: &str) -> serde_json::Value {
    chunks::role_chunk(model)
}

fn finish_chunk(model: &str, reason: &str) -> serde_json::Value {
    chunks::finish_chunk(model, reason)
}
