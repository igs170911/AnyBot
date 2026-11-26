use serde_json::Value;

use crate::http_wire::{WireHeader, WireRequest};
use crate::providers::OPENROUTER_APP;
use crate::thinking::apply_thinking;
use crate::types::{ProviderEndpoint, ProviderKind, TurnRequest, WireFormat};
use crate::{GatewayFail, wire_claude, wire_openai};

pub(crate) fn encode_body(
    endpoint: &ProviderEndpoint,
    request: &TurnRequest,
    stream: bool,
) -> Value {
    let mut body = match endpoint.format {
        WireFormat::Anthropic => wire_claude::encode(request, &endpoint.model, stream),
        WireFormat::OpenAiCompatible => wire_openai::encode(request, &endpoint.model, stream, true),
    };
    let _applied = apply_thinking(&mut body, endpoint.kind, endpoint.format, request.thinking);
    body
}

pub(crate) fn classified_status(
    endpoint: &ProviderEndpoint,
    bytes: &crate::transport::HttpBytes,
) -> GatewayFail {
    let mut fail = crate::ask::from_status(
        bytes.status,
        &String::from_utf8_lossy(&bytes.body),
        endpoint.kind,
        endpoint
            .api_key
            .as_ref()
            .map(anybot_foundation::Secret::expose),
        bytes.retry_after,
    );
    fail.classified.retry_after = fail.retry_after;
    crate::ask::classified_fail(fail.classified, fail.notices)
}

pub(crate) fn wire_for(url: &str, body: &Value, endpoint: &ProviderEndpoint) -> WireRequest {
    wire_request(url, body, endpoint)
}

pub(crate) fn wire_request(url: &str, body: &Value, endpoint: &ProviderEndpoint) -> WireRequest {
    let encoded = if body.as_object().is_some_and(serde_json::Map::is_empty) {
        String::new()
    } else {
        body.to_string()
    };
    WireRequest {
        method: if encoded.is_empty() {
            crate::http_wire::WireMethod::Get
        } else {
            crate::http_wire::WireMethod::Post
        },
        url: url.to_owned(),
        headers: auth_headers(endpoint),
        body: encoded,
        dial: None,
    }
}

fn header(name: &str, value: String) -> WireHeader {
    WireHeader {
        name: name.to_owned(),
        value: anybot_foundation::Secret::from_untrusted(value),
    }
}

fn auth_headers(endpoint: &ProviderEndpoint) -> Vec<WireHeader> {
    let mut headers = Vec::new();
    if let Some(key) = &endpoint.api_key {
        match endpoint.format {
            WireFormat::Anthropic => {
                headers.push(header("x-api-key", key.expose().to_owned()));
                headers.push(header(
                    "anthropic-version",
                    wire_claude::ANTHROPIC_VERSION.to_owned(),
                ));
            }
            WireFormat::OpenAiCompatible => {
                headers.push(header("authorization", format!("Bearer {}", key.expose())));
            }
        }
    }
    if matches!(endpoint.kind, ProviderKind::OpenRouter) {
        headers.push(header("HTTP-Referer", "https://anybot.local".to_owned()));
        headers.push(header("X-Title", OPENROUTER_APP.to_owned()));
    }
    headers
}
