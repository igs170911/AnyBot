use std::sync::Arc;

use anybot_foundation::clock::Clock;
use serde_json::Value;

use crate::egress::{ModelUse, SendPermit};
use crate::http_wire::{SharedWire, WireRequest};
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
    pub fn new(clock: Arc<dyn Clock>) -> Result<Self, String> {
        let client = crate::client_http::product_client()?;
        Ok(Self::from_wire(
            crate::reqwest_wire::ReqwestWire::new(client),
            clock,
        ))
    }

    pub(crate) fn from_wire(wire: SharedWire, clock: Arc<dyn Clock>) -> Self {
        Self {
            transport: TimedTransport::new(wire, clock),
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
            transport: TimedTransport::new(self.transport.wire(), self.transport.clock()),
            name_map: Arc::clone(&self.name_map),
        }
    }

    #[must_use]
    pub fn clone_with_names(&self, name_map: Arc<dyn NameResolver>) -> Self {
        Self {
            transport: TimedTransport::new(self.transport.wire(), self.transport.clock()),
            name_map,
        }
    }

    pub async fn complete(
        &self,
        endpoint: &ProviderEndpoint,
        request: &TurnRequest,
        stream: bool,
        permit: &SendPermit,
    ) -> Result<TurnResult, GatewayFail> {
        if endpoint.api_key.is_none() && crate::client_http::needs_key(endpoint.kind) {
            return Err(GatewayFail::MissingKey);
        }
        let url = crate::client_http::chat_url(endpoint);
        if !permit.matches(&url, ModelUse::Bot) {
            return Err(policy_blocked());
        }
        if endpoint.api_key.is_some() && !key_allowed(endpoint, &url) {
            return Err(GatewayFail::KeyDestination);
        }
        let body = encode_body(endpoint, request, stream);
        let wire = self.client_request(&url, &body, endpoint);
        let first_wait = first_token_wait(0);
        if stream {
            return self.complete_stream(endpoint, &wire, first_wait).await;
        }
        match self.transport.exchange(&wire, first_wait).await {
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
        endpoint: &ProviderEndpoint,
    ) -> WireRequest {
        let target = self.name_map.resolve(url).unwrap_or_else(|| url.to_owned());
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
            url: target,
            headers: auth_headers(endpoint),
            body: encoded,
        }
    }

    async fn complete_stream(
        &self,
        endpoint: &ProviderEndpoint,
        request: &WireRequest,
        first_wait: std::time::Duration,
    ) -> Result<TurnResult, GatewayFail> {
        let open = self
            .transport
            .open_stream(request, first_wait)
            .await
            .map_err(GatewayFail::Transport)?;
        if open.status >= 300 && open.status < 400 {
            return Err(crate::ask::classified_fail(
                crate::errors::classify(&crate::errors::FailInput::Redirect),
                Vec::new(),
            ));
        }
        if open.status >= 400 {
            return Err(classified_status(
                endpoint,
                &crate::transport::HttpBytes {
                    status: open.status,
                    body: bytes::Bytes::new(),
                    retry_after: crate::transport::retry_after_of(&open.headers),
                },
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
        request: &WireRequest,
        first_wait: std::time::Duration,
    ) -> Result<crate::transport::HttpBytes, crate::transport::TransportFail> {
        self.transport.exchange(request, first_wait).await
    }
}

pub(crate) fn chat_url_of(endpoint: &ProviderEndpoint) -> String {
    crate::client_http::chat_url(endpoint)
}

fn policy_blocked() -> GatewayFail {
    crate::ask::classified_fail(
        crate::errors::classify(&crate::errors::FailInput::PolicyBlocked),
        Vec::new(),
    )
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

fn auth_headers(endpoint: &ProviderEndpoint) -> Vec<(String, String)> {
    let mut headers = Vec::new();
    if let Some(key) = &endpoint.api_key {
        match endpoint.format {
            WireFormat::Anthropic => {
                headers.push(("x-api-key".to_owned(), key.expose().to_owned()));
                headers.push((
                    "anthropic-version".to_owned(),
                    wire_claude::ANTHROPIC_VERSION.to_owned(),
                ));
            }
            WireFormat::OpenAiCompatible => {
                headers.push((
                    "authorization".to_owned(),
                    format!("Bearer {}", key.expose()),
                ));
            }
        }
    }
    if matches!(endpoint.kind, ProviderKind::OpenRouter) {
        headers.push(("HTTP-Referer".to_owned(), "https://anybot.local".to_owned()));
        headers.push(("X-Title".to_owned(), OPENROUTER_APP.to_owned()));
    }
    headers
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
