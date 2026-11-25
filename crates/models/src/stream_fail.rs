use crate::errors::{Classified, FailInput, classify};
use crate::types::{ProviderEndpoint, StopKind, TurnResult, WireFormat};
use crate::{wire_claude, wire_openai};

pub(crate) fn finish_stream(
    endpoint: &ProviderEndpoint,
    raw: &str,
    saw_fail: bool,
) -> Result<TurnResult, crate::GatewayFail> {
    let events = sse_payloads(raw);
    if let Some(fail) = event_error(&events) {
        return Err(crate::ask::classified_fail(fail, Vec::new()));
    }
    let closed = !saw_fail && stream_closed(raw, endpoint.format);
    Ok(decode_events(endpoint, &events, closed))
}

pub(crate) fn partial_outcome(
    endpoint: &ProviderEndpoint,
    bytes: &[u8],
) -> Result<TurnResult, crate::GatewayFail> {
    let text = String::from_utf8_lossy(bytes);
    let events = sse_payloads(&text);
    if let Some(fail) = event_error(&events) {
        return Err(crate::ask::classified_fail(fail, Vec::new()));
    }
    Ok(interrupted(endpoint, &text))
}

pub(crate) fn saw_model_text(endpoint: &ProviderEndpoint, raw: &str) -> bool {
    let result = decode_events(endpoint, &sse_payloads(raw), false);
    !result.text.trim().is_empty()
        || !result.thinking.trim().is_empty()
        || !result.tool_calls.is_empty()
}

pub(crate) fn stream_closed(raw: &str, format: WireFormat) -> bool {
    match format {
        WireFormat::OpenAiCompatible => sse_done(raw),
        WireFormat::Anthropic => sse_payloads(raw).iter().any(|event| message_stop(event)),
    }
}

fn sse_done(raw: &str) -> bool {
    raw.split("\n\n").any(|block| {
        block.lines().any(|line| {
            line.strip_prefix("data:")
                .is_some_and(|data| data.trim() == "[DONE]")
        })
    })
}

fn message_stop(event: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(event)
        .ok()
        .and_then(|value| {
            value
                .get("type")
                .and_then(|kind| kind.as_str())
                .map(str::to_owned)
        })
        .is_some_and(|kind| kind == "message_stop")
}

pub(crate) fn sse_payloads(raw: &str) -> Vec<String> {
    raw.split("\n\n")
        .filter_map(|block| {
            let data = block.lines().find_map(|line| line.strip_prefix("data:"))?;
            let trimmed = data.trim();
            if trimmed.is_empty() || trimmed == "[DONE]" {
                return None;
            }
            Some(trimmed.to_owned())
        })
        .collect()
}

fn event_error(events: &[String]) -> Option<Classified> {
    let error = events.iter().find_map(|event| {
        let value = serde_json::from_str::<serde_json::Value>(event).ok()?;
        error_value(&value)
    })?;
    let lowered = error.to_ascii_lowercase();
    let input = if busy_stream(&lowered) {
        FailInput::StreamBusy
    } else {
        FailInput::StreamOther
    };
    Some(classify(&input))
}

fn error_value(value: &serde_json::Value) -> Option<String> {
    if value.get("type").and_then(|kind| kind.as_str()) == Some("error") {
        return Some(value.to_string());
    }
    value.get("error").map(serde_json::Value::to_string)
}

fn busy_stream(lowered: &str) -> bool {
    lowered.contains("engine_busy")
        || lowered.contains("overloaded_error")
        || lowered.contains("\"busy\"")
        || lowered.contains("too busy")
}

fn interrupted(endpoint: &ProviderEndpoint, body: &str) -> TurnResult {
    let events = sse_payloads(body);
    if events.is_empty() {
        let mut result = decode_body(endpoint, body);
        result.stop = StopKind::Interrupted;
        result.tool_calls.clear();
        return result;
    }
    decode_events(endpoint, &events, false)
}

pub(crate) fn decode_body(endpoint: &ProviderEndpoint, body: &str) -> TurnResult {
    match endpoint.format {
        WireFormat::Anthropic => wire_claude::decode(body, &endpoint.model),
        WireFormat::OpenAiCompatible => wire_openai::decode(body, endpoint.kind, &endpoint.model),
    }
}

fn decode_events(endpoint: &ProviderEndpoint, events: &[String], closed: bool) -> TurnResult {
    match endpoint.format {
        WireFormat::Anthropic => wire_claude::decode_stream(events, &endpoint.model, closed),
        WireFormat::OpenAiCompatible => {
            wire_openai::decode_stream(events, endpoint.kind, &endpoint.model, closed)
        }
    }
}
