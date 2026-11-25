use std::sync::Arc;

use anybot_foundation::clock::Clock;

use crate::egress::SendPermit;
use crate::http_wire::{SharedWire, WireRequest};
use crate::providers::key_allowed;
use crate::transport::TimedTransport;
use crate::types::{ProviderEndpoint, TurnRequest, TurnResult};
use crate::where_run::first_token_wait;

pub struct Gateway {
    transport: TimedTransport,
    name_map: Arc<dyn NameResolver>,
    os_notes: Vec<String>,
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
    pub fn new(
        clock: Arc<dyn Clock>,
        trusted: &anybot_foundation::TrustedCerts,
    ) -> Result<Self, String> {
        Self::with_proxy(clock, trusted, None)
    }

    pub fn with_proxy(
        clock: Arc<dyn Clock>,
        trusted: &anybot_foundation::TrustedCerts,
        proxy: Option<&str>,
    ) -> Result<Self, String> {
        let built = crate::client_http::product_client(trusted, proxy)?;
        Ok(Self::from_client(clock, built))
    }

    pub fn with_pinned_host(
        clock: Arc<dyn Clock>,
        trusted: &anybot_foundation::TrustedCerts,
        host: &str,
        addr: std::net::SocketAddr,
    ) -> Result<Self, String> {
        let built = crate::client_http::client_for(trusted, Some((host, addr)), None)?;
        Ok(Self::from_client(clock, built))
    }

    fn from_client(clock: Arc<dyn Clock>, built: crate::client_http::BuiltClient) -> Self {
        Self::from_wire(
            crate::reqwest_wire::ReqwestWire::new(built.client),
            clock,
            built.os_notes,
        )
    }

    pub(crate) fn from_wire(
        wire: SharedWire,
        clock: Arc<dyn Clock>,
        os_notes: Vec<String>,
    ) -> Self {
        Self {
            transport: TimedTransport::new(wire, clock),
            name_map: Arc::new(IdentityResolver),
            os_notes,
        }
    }

    #[must_use]
    pub fn os_notes(&self) -> &[String] {
        &self.os_notes
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
            os_notes: self.os_notes.clone(),
        }
    }

    #[must_use]
    pub fn clone_with_names(&self, name_map: Arc<dyn NameResolver>) -> Self {
        Self {
            transport: TimedTransport::new(self.transport.wire(), self.transport.clock()),
            name_map,
            os_notes: self.os_notes.clone(),
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
        let listed = crate::client_http::chat_url(endpoint);
        let use_kind = permit.use_kind();
        if !permit.matches(&listed, use_kind) {
            return Err(policy_blocked());
        }
        if endpoint.api_key.is_some() && !key_allowed(endpoint, &listed) {
            return Err(GatewayFail::KeyDestination);
        }
        let url = self.resolved_url(&listed);
        let body = crate::client_wire::encode_body(endpoint, request, stream);
        let wire = crate::client_wire::wire_request(&url, &body, endpoint);
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
                    return Err(crate::client_wire::classified_status(endpoint, &bytes));
                }
                Ok(crate::stream_fail::decode_body(
                    endpoint,
                    &String::from_utf8_lossy(&bytes.body),
                ))
            }
            Err(crate::transport::TransportFail::Partial(partial)) => {
                crate::stream_fail::partial_outcome(endpoint, &partial.bytes)
            }
            Err(other) => Err(crate::ask::transport_fail(endpoint, &other, false)),
        }
    }

    fn resolved_url(&self, url: &str) -> String {
        self.name_map.resolve(url).unwrap_or_else(|| url.to_owned())
    }

    pub(crate) fn resolved_url_of(&self, url: &str) -> String {
        self.resolved_url(url)
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
            .map_err(|fail| crate::ask::transport_fail(endpoint, &fail, false))?;
        if open.status >= 300 && open.status < 400 {
            return Err(crate::ask::classified_fail(
                crate::errors::classify(&crate::errors::FailInput::Redirect),
                Vec::new(),
            ));
        }
        let mut raw = Vec::new();
        let mut rx = open.rx;
        let mut saw_fail = false;
        while let Some(item) = rx.recv().await {
            match item {
                Ok(chunk) => raw.extend_from_slice(&chunk),
                Err(crate::transport::TransportFail::Partial(partial)) => {
                    if raw.is_empty() {
                        raw = partial.bytes;
                    }
                    saw_fail = true;
                    break;
                }
                Err(crate::transport::TransportFail::FirstToken) if raw.is_empty() => {
                    return Err(crate::ask::transport_fail(
                        endpoint,
                        &crate::transport::TransportFail::FirstToken,
                        false,
                    ));
                }
                Err(_) => {
                    saw_fail = true;
                    break;
                }
            }
        }
        finish_open(endpoint, open.status, &open.headers, &raw, saw_fail)
    }

    pub(crate) async fn transport_send(
        &self,
        request: &WireRequest,
        first_wait: std::time::Duration,
    ) -> Result<crate::transport::HttpBytes, crate::transport::TransportFail> {
        self.transport.exchange(request, first_wait).await
    }
}

fn finish_open(
    endpoint: &ProviderEndpoint,
    status: u16,
    headers: &[(String, String)],
    raw: &[u8],
    saw_fail: bool,
) -> Result<TurnResult, GatewayFail> {
    let text = String::from_utf8_lossy(raw);
    if status >= 400 {
        return Err(crate::client_wire::classified_status(
            endpoint,
            &crate::transport::HttpBytes {
                status,
                body: bytes::Bytes::from(text.into_owned()),
                retry_after: crate::transport::retry_after_of(headers),
            },
        ));
    }
    if saw_fail && !crate::stream_fail::saw_model_text(endpoint, &text) {
        return Err(crate::ask::transport_fail(
            endpoint,
            &crate::transport::TransportFail::Dropped,
            false,
        ));
    }
    crate::stream_fail::finish_stream(endpoint, &text, saw_fail)
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
