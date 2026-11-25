use crate::errors::{Classified, FailInput, classify};
use crate::types::{ProviderEndpoint, StopKind, TurnResult, WireFormat};
use crate::{wire_claude, wire_openai};

pub(crate) fn finish_stream(
    endpoint: &ProviderEndpoint,
    raw: &str,
    saw_fail: bool,
) -> Result<TurnResult, crate::GatewayFail> {
    if let Some(fail) = stream_error_fail(raw) {
        return Err(crate::ask::classified_fail(fail, Vec::new()));
    }
    let closed = !saw_fail && stream_closed(raw);
    Ok(decode_events(endpoint, &sse_payloads(raw), closed))
}

pub(crate) fn partial_outcome(
    endpoint: &ProviderEndpoint,
    bytes: &[u8],
) -> Result<TurnResult, crate::GatewayFail> {
    let text = String::from_utf8_lossy(bytes);
    if let Some(fail) = stream_error_fail(&text) {
        return Err(crate::ask::classified_fail(fail, Vec::new()));
    }
    Ok(interrupted(endpoint, &text))
}

pub(crate) fn stream_closed(raw: &str) -> bool {
    raw.contains("[DONE]") || raw.contains("message_stop")
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

fn stream_error_fail(raw: &str) -> Option<Classified> {
    let lowered = raw.to_ascii_lowercase();
    if !lowered.contains("\"error\"") {
        return None;
    }
    let input = if busy_stream(&lowered) {
        FailInput::StreamBusy
    } else {
        FailInput::StreamOther
    };
    Some(classify(&input))
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
