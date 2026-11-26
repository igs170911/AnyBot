use serde_json::Value;

use crate::types::UsageReport;
use crate::wire_openai_calls::{PartialCall, append_calls};
use crate::wire_openai_usage::usage_of;

pub(crate) fn apply_openai_chunk(
    value: &Value,
    text: &mut String,
    thinking: &mut String,
    calls: &mut Vec<PartialCall>,
    finish: &mut String,
    usage: &mut UsageReport,
) {
    if let Some(reported) = usage_of(value) {
        *usage = UsageReport::Present(reported);
    }
    let Some(choice) = value.get("choices").and_then(|c| c.get(0)) else {
        return;
    };
    if let Some(reason) = choice.get("finish_reason").and_then(Value::as_str) {
        crate::jsonutil::set_owned(finish, reason);
    }
    let Some(delta) = choice.get("delta").or_else(|| choice.get("message")) else {
        return;
    };
    append_text(text, delta.get("content"));
    append_text(thinking, delta.get("reasoning"));
    append_text(thinking, delta.get("reasoning_content"));
    append_calls(calls, delta.get("tool_calls"));
}

pub(crate) fn parse_chunk(chunk: &str) -> Option<Value> {
    if let Ok(value) = serde_json::from_str::<Value>(chunk) {
        return Some(value);
    }
    let end = chunk.rfind('}')?;
    serde_json::from_str::<Value>(chunk.get(..=end)?).ok()
}

pub(crate) fn append_text(into: &mut String, value: Option<&Value>) {
    let Some(value) = value else {
        return;
    };
    if let Some(text) = value.as_str() {
        into.push_str(text);
        return;
    }
    if let Some(text) = value.get("content").and_then(Value::as_str) {
        into.push_str(text);
    }
}
