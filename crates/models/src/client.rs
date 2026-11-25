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

    #[must_use]
    pub fn clone_gateway(&self) -> Self {
        Self {
            transport: TimedTransport::new(self.transport.client.clone(), self.transport.clock()),
            name_map: Arc::clone(&self.name_map),
        }
    }

    #[must_use]
    pub fn clone_with_names(&self, name_map: Arc<dyn NameResolver>) -> Self {
        Self {
            transport: TimedTransport::new(self.transport.client.clone(), self.transport.clock()),
            name_map,
        }
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
                if bytes.status >= 300 && bytes.status < 400 {
                    return Err(crate::ask::classified_fail(
                        crate::errors::classify(&crate::errors::FailInput::Redirect),
                        Vec::new(),
                    ));
                }
                if bytes.status >= 400 {
                    return Err(classified_status(endpoint, &bytes));
                }
                Ok(crate::stream_fail::decode_body(
                    endpoint,
                    &String::from_utf8_lossy(&bytes.body),
                ))
            }
            Err(crate::transport::TransportFail::Partial(partial)) => {
                crate::stream_fail::partial_outcome(endpoint, &partial.bytes)
            }
            Err(other) => Err(GatewayFail::Transport(other)),
        }
    }

    pub(crate) fn client_request(
        &self,
        url: &str,
        body: &Value,
        _endpoint: &ProviderEndpoint,
    ) -> reqwest::RequestBuilder {
        let target = self.name_map.resolve(url).unwrap_or_else(|| url.to_owned());
        if body.as_object().is_some_and(serde_json::Map::is_empty) {
            return self.transport.client.get(target);
        }
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
            return Err(crate::ask::classified_fail(
                crate::errors::classify(&crate::errors::FailInput::Http {
                    status: open.status,
                    message: String::new(),
                }),
                Vec::new(),
            ));
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
        crate::stream_fail::finish_stream(endpoint, &raw, saw_fail)
    }

    pub(crate) async fn transport_send(
        &self,
        builder: reqwest::RequestBuilder,
        first_wait: std::time::Duration,
    ) -> Result<crate::transport::HttpBytes, crate::transport::TransportFail> {
        self.transport.send(builder, first_wait).await
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

fn classified_status(
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

#[derive(Debug)]
pub enum GatewayFail {
    MissingKey,
    KeyDestination,
    Status(u16),
    Transport(crate::transport::TransportFail),
    Stopped,
    Classified {
        classified: crate::errors::Classified,
        notices: Vec<crate::notice::Notice>,
    },
}
