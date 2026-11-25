use std::sync::Arc;

use anybot_foundation::clock::Clock;
use serde_json::Value;

use crate::providers::{OPENROUTER_APP, key_allowed};
use crate::thinking::apply_thinking;
use crate::transport::TimedTransport;
use crate::types::{ProviderEndpoint, ProviderKind, TurnRequest, TurnResult, WireFormat};
use crate::where_run::first_token_wait;
use crate::{wire_claude, wire_openai};

pub struct Gateway {
    transport: TimedTransport,
    name_map: Arc<dyn NameResolver>,
}

pub trait NameResolver: Send + Sync {
    fn resolve(&self, url: &str) -> Option<String>;
}

pub struct IdentityResolver;

impl NameResolver for IdentityResolver {
    fn resolve(&self, _url: &str) -> Option<String> {
        None
    }
}

impl Gateway {
    pub fn new(client: reqwest::Client, clock: Arc<dyn Clock>) -> Self {
        Self {
            transport: TimedTransport::new(client, clock),
            name_map: Arc::new(IdentityResolver),
        }
    }

    #[must_use]
    pub fn with_names(mut self, name_map: Arc<dyn NameResolver>) -> Self {
        self.name_map = name_map;
        self
    }

    pub async fn complete(
        &self,
        endpoint: &ProviderEndpoint,
        request: &TurnRequest,
        stream: bool,
    ) -> Result<TurnResult, GatewayFail> {
        if endpoint.api_key.is_none() && needs_key(endpoint.kind) {
            return Err(GatewayFail::MissingKey);
        }
        let url = Self::chat_url(endpoint);
        if endpoint.api_key.is_some() && !key_allowed(endpoint, &url) {
            return Err(GatewayFail::KeyDestination);
        }
        let body = encode_body(endpoint, request, stream);
        let builder = authorized(self.client_request(&url, &body, endpoint), endpoint);
        let first_wait = first_token_wait(0);
        if stream {
            return self.complete_stream(endpoint, builder, first_wait).await;
        }
        match self.transport.send(builder, first_wait).await {
            Ok(bytes) => {
                if bytes.status >= 300 {
                    return Err(GatewayFail::Status(bytes.status));
                }
                Ok(decode_body(endpoint, &String::from_utf8_lossy(&bytes.body)))
            }
            Err(crate::transport::TransportFail::Partial(partial)) => Ok(interrupted(
                endpoint,
                &String::from_utf8_lossy(&partial.bytes),
            )),
            Err(other) => Err(GatewayFail::Transport(other)),
        }
    }

    fn client_request(
        &self,
        url: &str,
        body: &Value,
        _endpoint: &ProviderEndpoint,
    ) -> reqwest::RequestBuilder {
        let target = self.name_map.resolve(url).unwrap_or_else(|| url.to_owned());
        self.transport.client.post(target).json(body)
    }

    async fn complete_stream(
        &self,
        endpoint: &ProviderEndpoint,
        builder: reqwest::RequestBuilder,
        first_wait: std::time::Duration,
    ) -> Result<TurnResult, GatewayFail> {
        let open = self
            .transport
            .open_stream(builder, first_wait)
            .await
            .map_err(GatewayFail::Transport)?;
        if open.status >= 300 {
            return Err(GatewayFail::Status(open.status));
        }
        let mut raw = String::new();
        let mut rx = open.rx;
        let mut saw_fail = false;
        while let Some(item) = rx.recv().await {
            match item {
                Ok(chunk) => raw.push_str(&String::from_utf8_lossy(&chunk)),
                Err(crate::transport::TransportFail::Partial(partial)) => {
                    if raw.is_empty() {
                        raw.push_str(&String::from_utf8_lossy(&partial.bytes));
                    }
                    saw_fail = true;
                    break;
                }
                Err(_) => {
                    saw_fail = true;
                    break;
                }
            }
        }
        let closed = !saw_fail && stream_closed(&raw);
        let events = sse_payloads(&raw);
        Ok(decode_events(endpoint, &events, closed))
    }

    fn chat_url(endpoint: &ProviderEndpoint) -> String {
        let path = match endpoint.format {
            WireFormat::Anthropic => wire_claude::MESSAGES_PATH,
            WireFormat::OpenAiCompatible => wire_openai::CHAT_PATH,
        };
        format!("{}{path}", endpoint.base_url)
    }
}

fn needs_key(kind: ProviderKind) -> bool {
    !matches!(kind, ProviderKind::Ollama | ProviderKind::Custom)
}

fn encode_body(endpoint: &ProviderEndpoint, request: &TurnRequest, stream: bool) -> Value {
    let mut body = match endpoint.format {
        WireFormat::Anthropic => wire_claude::encode(request, &endpoint.model, stream),
        WireFormat::OpenAiCompatible => wire_openai::encode(request, &endpoint.model, stream, true),
    };
    let _applied = apply_thinking(&mut body, endpoint.kind, endpoint.format, request.thinking);
    body
}

fn interrupted(endpoint: &ProviderEndpoint, body: &str) -> TurnResult {
    let events = sse_payloads(body);
    if events.is_empty() {
        let mut result = decode_body(endpoint, body);
        result.stop = crate::types::StopKind::Interrupted;
        result.tool_calls.clear();
        return result;
    }
    decode_events(endpoint, &events, false)
}

fn decode_body(endpoint: &ProviderEndpoint, body: &str) -> TurnResult {
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

fn authorized(
    builder: reqwest::RequestBuilder,
    endpoint: &ProviderEndpoint,
) -> reqwest::RequestBuilder {
    let mut builder = builder;
    if let Some(key) = &endpoint.api_key {
        builder = match endpoint.format {
            WireFormat::Anthropic => builder
                .header("x-api-key", key.expose())
                .header("anthropic-version", wire_claude::ANTHROPIC_VERSION),
            WireFormat::OpenAiCompatible => builder.bearer_auth(key.expose()),
        };
    }
    if matches!(endpoint.kind, ProviderKind::OpenRouter) {
        builder = builder
            .header("HTTP-Referer", "https://anybot.local")
            .header("X-Title", OPENROUTER_APP);
    }
    builder
}

fn stream_closed(raw: &str) -> bool {
    raw.contains("[DONE]") || raw.contains("message_stop")
}

fn sse_payloads(raw: &str) -> Vec<String> {
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

#[derive(Debug)]
pub enum GatewayFail {
    MissingKey,
    KeyDestination,
    Status(u16),
    Transport(crate::transport::TransportFail),
}
