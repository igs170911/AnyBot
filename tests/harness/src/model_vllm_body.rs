use serde_json::json;

use crate::model_extra::openai_usage;
use crate::model_openai::{FAKE_CREATED, FAKE_FINGERPRINT};
use crate::model_script::Step;
use crate::model_vllm::VLLM_TOOL_FINISH;

#[must_use]
pub fn json_body(step: &Step, model: &str) -> String {
    let (message, reason) = completion_message(step);
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
            "stop_reason": null,
            "token_ids": null,
            "routed_experts": null,
            "weight_versions": null,
        }],
        "usage": vllm_usage(step),
        "service_tier": null,
        "system_fingerprint": FAKE_FINGERPRINT,
        "prompt_logprobs": null,
        "prompt_text": null,
        "prompt_token_ids": null,
        "kv_transfer_params": null,
        "ec_transfer_params": null,
        "metrics": null,
    });
    body.to_string()
}

fn completion_message(step: &Step) -> (serde_json::Value, String) {
    match step {
        Step::ReplyText { text, .. } | Step::UsageReply { text, .. } => {
            (plain_message(text, ""), VLLM_TOOL_FINISH.to_owned())
        }
        Step::ThinkThenReply { thinking, text, .. }
        | Step::ThinkingBlock { thinking, text, .. } => {
            (plain_message(text, thinking), VLLM_TOOL_FINISH.to_owned())
        }
        Step::CallTool { name, args, .. } => (
            vllm_tool_message(&crate::model_script::ToolCallScript::named(
                name,
                &args.to_string(),
            )),
            VLLM_TOOL_FINISH.to_owned(),
        ),
        Step::ToolCalls {
            calls,
            finish_reason,
            ..
        } => (vllm_tools_message(calls), finish_reason.clone()),
        Step::ThinkThenTool {
            thinking, calls, ..
        } => (
            crate::model_vllm::vllm_think_tools(thinking, calls),
            VLLM_TOOL_FINISH.to_owned(),
        ),
        Step::StopReason {
            text, stop_reason, ..
        } => (plain_message(text, ""), stop_reason.clone()),
        Step::NoResponse { .. }
        | Step::CutOffAfter { .. }
        | Step::StallAfter { .. }
        | Step::HoldThenFinish { .. }
        | Step::ReplyStatus { .. }
        | Step::StreamError { .. }
        | Step::EmptyBody
        | Step::Redirect { .. }
        | Step::ModelList { .. }
        | Step::DelayHeaders { .. }
        | Step::AcceptThenDelay { .. } => (plain_message("", ""), VLLM_TOOL_FINISH.to_owned()),
    }
}

fn vllm_tool_message(call: &crate::model_script::ToolCallScript) -> serde_json::Value {
    vllm_tools_message(std::slice::from_ref(call))
}

pub(crate) fn usage_chunk(model: &str) -> serde_json::Value {
    json!({"id": "chatcmpl-fake", "object": "chat.completion.chunk", "created": FAKE_CREATED, "model": model, "choices": [], "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}, "system_fingerprint": FAKE_FINGERPRINT})
}

pub(crate) fn vllm_tools_message(
    calls: &[crate::model_script::ToolCallScript],
) -> serde_json::Value {
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
        "function_call": null,
        "reasoning": "",
        "tool_calls": tool_calls,
    })
}

fn vllm_usage(step: &Step) -> serde_json::Value {
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
        | Step::HoldThenFinish { .. }
        | Step::ReplyStatus { .. }
        | Step::StreamError { .. }
        | Step::EmptyBody
        | Step::Redirect { .. }
        | Step::ModelList { .. }
        | Step::DelayHeaders { .. }
        | Step::AcceptThenDelay { .. }
        | Step::UsageReply { .. }
        | Step::ThinkingBlock { .. }
        | Step::StopReason { .. }
        | Step::ToolCalls { .. }
        | Step::ThinkThenTool { .. } => {
            json!({ "prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2 })
        }
    }
}

fn plain_message(text: &str, thinking: &str) -> serde_json::Value {
    json!({
        "role": "assistant",
        "content": text,
        "refusal": null,
        "audio": null,
        "function_call": null,
        "reasoning": thinking,
    })
}
