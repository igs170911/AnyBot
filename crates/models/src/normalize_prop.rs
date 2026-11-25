use crate::types::{ProviderKind, StopKind, TurnResult, WireFormat};
use crate::{wire_claude, wire_openai};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fixture {
    pub format: WireFormat,
    pub text: String,
    pub thinking: String,
    pub tool_name: String,
    pub tool_args: String,
    pub finish: String,
    pub cut: bool,
    pub extra: String,
}

#[must_use]
pub fn normalize_pair(fixture: &Fixture) -> (TurnResult, TurnResult) {
    let model = "m";
    let provider = match fixture.format {
        WireFormat::Anthropic => ProviderKind::Anthropic,
        WireFormat::OpenAiCompatible => ProviderKind::Custom,
    };
    let events = encode_events(fixture);
    let whole = decode_events(&events, fixture, provider, model, true);
    let cut_at = cut_index(events.len(), fixture.cut);
    let partial = decode_events(
        events.get(..cut_at).unwrap_or(&events),
        fixture,
        provider,
        model,
        !fixture.cut,
    );
    (whole, partial)
}

fn cut_index(len: usize, cut: bool) -> usize {
    if !cut || len == 0 {
        return len;
    }
    len.saturating_sub(1).max(1).min(len)
}

fn decode_events(
    events: &[String],
    fixture: &Fixture,
    provider: ProviderKind,
    model: &str,
    closed: bool,
) -> TurnResult {
    match fixture.format {
        WireFormat::Anthropic => wire_claude::decode_stream(events, model, closed),
        WireFormat::OpenAiCompatible => wire_openai::decode_stream(events, provider, model, closed),
    }
}

fn encode_events(fixture: &Fixture) -> Vec<String> {
    match fixture.format {
        WireFormat::Anthropic => anthropic_events(fixture),
        WireFormat::OpenAiCompatible => openai_events(fixture),
    }
}

fn openai_events(fixture: &Fixture) -> Vec<String> {
    let mut events = vec![event(&serde_json::json!({
        "choices": [{ "delta": { "role": "assistant" }, "finish_reason": null }],
        "extra_field": fixture.extra,
    }))];
    if !fixture.thinking.is_empty() {
        events.push(event(&serde_json::json!({
            "choices": [{ "delta": { "reasoning": fixture.thinking }, "finish_reason": null }]
        })));
    }
    if !fixture.text.is_empty() {
        events.push(event(&serde_json::json!({
            "choices": [{ "delta": { "content": fixture.text }, "finish_reason": null }]
        })));
    }
    if !fixture.tool_name.is_empty() {
        events.push(event(&serde_json::json!({
            "choices": [{
                "delta": { "tool_calls": [{
                    "index": 0,
                    "id": "call_1",
                    "function": { "name": fixture.tool_name, "arguments": fixture.tool_args }
                }] },
                "finish_reason": null
            }]
        })));
    }
    events.push(event(&serde_json::json!({
        "choices": [{ "delta": {}, "finish_reason": openai_finish(fixture) }]
    })));
    events
}

fn openai_finish(fixture: &Fixture) -> &str {
    if !fixture.tool_name.is_empty() {
        "tool_calls"
    } else if fixture.finish == "length" {
        "length"
    } else {
        "stop"
    }
}

fn anthropic_events(fixture: &Fixture) -> Vec<String> {
    let mut events = Vec::new();
    let mut index = 0_u64;
    if !fixture.thinking.is_empty() {
        events.push(event(&serde_json::json!({
            "type": "content_block_start",
            "index": index,
            "content_block": { "type": "thinking", "thinking": "", "signature": "sig" },
            "extra_field": fixture.extra,
        })));
        events.push(event(&serde_json::json!({
            "type": "content_block_delta",
            "index": index,
            "delta": { "type": "thinking_delta", "thinking": fixture.thinking }
        })));
        index = index.saturating_add(1);
    }
    if !fixture.text.is_empty() {
        events.push(event(&serde_json::json!({
            "type": "content_block_start",
            "index": index,
            "content_block": { "type": "text", "text": "" }
        })));
        events.push(event(&serde_json::json!({
            "type": "content_block_delta",
            "index": index,
            "delta": { "type": "text_delta", "text": fixture.text }
        })));
        index = index.saturating_add(1);
    }
    if !fixture.tool_name.is_empty() {
        events.push(event(&serde_json::json!({
            "type": "content_block_start",
            "index": index,
            "content_block": {
                "type": "tool_use",
                "id": "toolu_1",
                "name": fixture.tool_name,
                "input": {}
            }
        })));
        events.push(event(&serde_json::json!({
            "type": "content_block_delta",
            "index": index,
            "delta": { "type": "input_json_delta", "partial_json": fixture.tool_args }
        })));
    }
    events.push(event(&serde_json::json!({
        "type": "message_delta",
        "delta": { "stop_reason": anthropic_finish(fixture) }
    })));
    events.push(event(&serde_json::json!({ "type": "message_stop" })));
    events
}

fn anthropic_finish(fixture: &Fixture) -> &str {
    if !fixture.tool_name.is_empty() {
        "tool_use"
    } else if fixture.finish == "length" {
        "max_tokens"
    } else {
        "end_turn"
    }
}

fn event(value: &serde_json::Value) -> String {
    value.to_string()
}

#[must_use]
pub fn closed_tool_matches(result: &TurnResult) -> bool {
    if matches!(result.stop, StopKind::Interrupted | StopKind::Length) {
        return result.tool_calls.is_empty();
    }
    let complete = result.tool_calls.iter().any(|call| {
        !call.name.is_empty() && matches!(call.arguments, crate::types::ToolArguments::Json(_))
    });
    matches!(result.stop, StopKind::NeedsTool) == complete || result.tool_calls.is_empty()
}

#[must_use]
pub fn cut_has_no_tools(result: &TurnResult) -> bool {
    !matches!(result.stop, StopKind::Interrupted) || result.tool_calls.is_empty()
}
