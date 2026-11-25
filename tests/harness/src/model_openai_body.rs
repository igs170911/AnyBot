use serde_json::json;

use super::{FAKE_CREATED, FAKE_FINGERPRINT};
use crate::model_extra::openai_usage;
use crate::model_script::{Step, ToolCallScript};

#[must_use]
pub fn json_body(step: &Step, model: &str) -> String {
    let (message, reason) = completion_message(step);
    let usage = usage_of(step);
    let body = json!({
        "id": "chatcmpl-fake",
        "object": "chat.completion",
        "created": FAKE_CREATED,
        "model": model,
        "choices": [{
            "index": 0,
            "message": message,
            "logprobs": null,
            "finish_reason": reason,
        }],
        "usage": usage,
        "service_tier": null,
        "system_fingerprint": FAKE_FINGERPRINT,
    });
    body.to_string()
}

fn usage_of(step: &Step) -> serde_json::Value {
    match step {
        Step::UsageReply {
            usage, omit_usage, ..
        } if !omit_usage => openai_usage(usage),
        Step::ReplyText { .. }
        | Step::ThinkThenReply { .. }
        | Step::CallTool { .. }
        | Step::NoResponse { .. }
        | Step::CutOffAfter { .. }
        | Step::StallAfter { .. }
        | Step::ReplyStatus { .. }
        | Step::StreamError { .. }
        | Step::EmptyBody
        | Step::Redirect { .. }
        | Step::ModelList { .. }
        | Step::UsageReply { .. }
        | Step::ThinkingBlock { .. }
        | Step::StopReason { .. }
        | Step::ToolCalls { .. }
        | Step::ThinkThenTool { .. } => {
            json!({ "prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2 })
        }
    }
}

fn completion_message(step: &Step) -> (serde_json::Value, String) {
    match step {
        Step::ReplyText { text, .. }
        | Step::UsageReply { text, .. }
        | Step::StopReason { text, .. } => (plain_message(text), stop_of(step)),
        Step::ThinkThenReply { thinking, text, .. }
        | Step::ThinkingBlock { thinking, text, .. } => {
            (reason_content(text, thinking), "stop".to_owned())
        }
        Step::CallTool { name, args, .. } => (
            tool_message(&ToolCallScript::named(name, &args.to_string())),
            "tool_calls".to_owned(),
        ),
        Step::ThinkThenTool { calls, .. } => (tools_message(calls), "tool_calls".to_owned()),
        Step::ToolCalls {
            calls,
            finish_reason,
            ..
        } => (tools_message(calls), finish_reason.clone()),
        Step::NoResponse { .. }
        | Step::CutOffAfter { .. }
        | Step::StallAfter { .. }
        | Step::ReplyStatus { .. }
        | Step::StreamError { .. }
        | Step::EmptyBody
        | Step::Redirect { .. }
        | Step::ModelList { .. } => (plain_message(""), "stop".to_owned()),
    }
}

fn stop_of(step: &Step) -> String {
    match step {
        Step::StopReason { stop_reason, .. } => stop_reason.clone(),
        Step::ReplyText { .. }
        | Step::ThinkThenReply { .. }
        | Step::CallTool { .. }
        | Step::NoResponse { .. }
        | Step::CutOffAfter { .. }
        | Step::StallAfter { .. }
        | Step::ReplyStatus { .. }
        | Step::StreamError { .. }
        | Step::EmptyBody
        | Step::Redirect { .. }
        | Step::ModelList { .. }
        | Step::UsageReply { .. }
        | Step::ThinkingBlock { .. }
        | Step::ToolCalls { .. }
        | Step::ThinkThenTool { .. } => "stop".to_owned(),
    }
}

fn tool_message(call: &ToolCallScript) -> serde_json::Value {
    tools_message(std::slice::from_ref(call))
}

fn tools_message(calls: &[ToolCallScript]) -> serde_json::Value {
    let tool_calls: Vec<serde_json::Value> = calls
        .iter()
        .map(|call| {
            json!({
                "id": call.id.clone().unwrap_or_default(),
                "type": "function",
                "function": { "name": call.name, "arguments": call.arguments }
            })
        })
        .collect();
    json!({
        "role": "assistant",
        "content": "",
        "refusal": null,
        "audio": null,
        "tool_calls": tool_calls,
    })
}

fn reason_content(text: &str, thinking: &str) -> serde_json::Value {
    let mut message = plain_message(text);
    if let Some(map) = message.as_object_mut() {
        map.insert("reasoning_content".to_owned(), json!(thinking));
    }
    message
}

fn plain_message(text: &str) -> serde_json::Value {
    json!({
        "role": "assistant",
        "content": text,
        "refusal": null,
        "audio": null,
    })
}
