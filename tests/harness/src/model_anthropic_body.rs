use serde_json::json;

use super::FAKE_MESSAGE_ID;
use crate::model_extra::anthropic_usage;
use crate::model_script::{Step, ToolCallScript};

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
        "usage": anthropic_usage_of(step),
    });
    body.to_string()
}

fn anthropic_usage_of(step: &Step) -> serde_json::Value {
    match step {
        Step::UsageReply {
            usage, omit_usage, ..
        } if !omit_usage => anthropic_usage(usage),
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
        | Step::UsageReply { .. }
        | Step::ThinkingBlock { .. }
        | Step::StopReason { .. }
        | Step::ToolCalls { .. }
        | Step::ThinkThenTool { .. } => json!({ "input_tokens": 1, "output_tokens": 1 }),
    }
}

fn content_blocks(step: &Step) -> (Vec<serde_json::Value>, String) {
    match step {
        Step::ReplyText { text, .. } | Step::UsageReply { text, .. } => {
            (vec![text_block(text)], "end_turn".to_owned())
        }
        Step::ThinkThenReply { thinking, text, .. } => (
            vec![
                json!({ "type": "thinking", "thinking": thinking, "signature": "sig-fake" }),
                text_block(text),
            ],
            "end_turn".to_owned(),
        ),
        Step::ThinkingBlock {
            thinking,
            signature,
            text,
            redacted,
            ..
        } => (
            thinking_blocks(thinking, signature, text, *redacted),
            "end_turn".to_owned(),
        ),
        Step::CallTool { name, args, .. } => (
            vec![json!({ "type": "tool_use", "id": "toolu_fake", "name": name, "input": args })],
            "tool_use".to_owned(),
        ),
        Step::ToolCalls {
            calls,
            finish_reason,
            ..
        } => (anthropic_tool_blocks(calls), finish_reason.clone()),
        Step::ThinkThenTool {
            thinking,
            signature,
            calls,
            ..
        } => (
            think_tool_blocks(thinking, signature, calls),
            "tool_use".to_owned(),
        ),
        Step::StopReason {
            text, stop_reason, ..
        } => (vec![text_block(text)], stop_reason.clone()),
        Step::NoResponse { .. }
        | Step::CutOffAfter { .. }
        | Step::StallAfter { .. }
        | Step::HoldThenFinish { .. }
        | Step::ReplyStatus { .. }
        | Step::StreamError { .. }
        | Step::EmptyBody
        | Step::Redirect { .. }
        | Step::ModelList { .. } => (Vec::new(), "end_turn".to_owned()),
    }
}

fn think_tool_blocks(
    thinking: &str,
    signature: &str,
    calls: &[ToolCallScript],
) -> Vec<serde_json::Value> {
    let mut blocks =
        vec![json!({ "type": "thinking", "thinking": thinking, "signature": signature })];
    blocks.extend(anthropic_tool_blocks(calls));
    blocks
}

fn thinking_blocks(
    thinking: &str,
    signature: &str,
    text: &str,
    redacted: bool,
) -> Vec<serde_json::Value> {
    let head = if redacted {
        json!({ "type": "redacted_thinking", "data": signature })
    } else {
        json!({ "type": "thinking", "thinking": thinking, "signature": signature })
    };
    vec![head, text_block(text)]
}

fn anthropic_tool_blocks(calls: &[ToolCallScript]) -> Vec<serde_json::Value> {
    calls
        .iter()
        .enumerate()
        .map(|(index, call)| {
            let id = call.id.clone().unwrap_or_else(|| format!("toolu_{index}"));
            let input = serde_json::from_str::<serde_json::Value>(&call.arguments)
                .unwrap_or(json!({ "raw": call.arguments }));
            json!({ "type": "tool_use", "id": id, "name": call.name, "input": input })
        })
        .collect()
}

fn text_block(text: &str) -> serde_json::Value {
    json!({ "type": "text", "text": text })
}
