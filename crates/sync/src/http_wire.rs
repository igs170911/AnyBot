use std::sync::Arc;

use anybot_foundation::{Clock, Secret, TrustStore, TrustedCerts};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderValue};

use crate::http_gate::{ConnectGate, GateLayer};
use crate::reconnect_direct::goes_direct;
use crate::reconnect_status::RequestGuard;
use crate::wire::{Wire, WireFailure, WireFuture, WireLimits, WireMethod, WireReply, WireRequest};

pub const CLIENT_VERSION_HEADER: &str = "anybot-client-version";
pub const IDEMPOTENCY_HEADER: &str = "idempotency-key";
pub const SERVER_TIME_HEADER: &str = "anybot-server-time";

pub struct HttpWire {
    base: url::Url,
    bearer: HeaderValue,
    version: HeaderValue,
    tls: rustls::ClientConfig,
    proxy: Option<url::Url>,
    clock: Arc<dyn Clock>,
}

impl HttpWire {
    pub fn new(
        base: &str,
        credential: &Secret,
        client_version: &str,
        trusted: &TrustedCerts,
        proxy: Option<&str>,
        clock: Arc<dyn Clock>,
    ) -> Result<Self, String> {
        let base = url::Url::parse(base).map_err(|e| format!("server url: {e}"))?;
        let store = TrustStore::new(trusted)?;
        let tls = store.client_config()?;
        let mut bearer = HeaderValue::from_str(&format!("Bearer {}", credential.expose()))
            .map_err(|_| "credential is not a valid header value".to_owned())?;
        bearer.set_sensitive(true);
        let version = HeaderValue::from_str(client_version)
            .map_err(|_| "client version is not a valid header value".to_owned())?;
        let proxy = proxy
            .map(|raw| url::Url::parse(raw).map_err(|e| format!("proxy url: {e}")))
            .transpose()?;
        Ok(HttpWire {
            base,
            bearer,
            version,
            tls,
            proxy,
            clock,
        })
    }

    fn client(
        &self,
        gate: &Arc<ConnectGate>,
        limits: WireLimits,
    ) -> Result<reqwest::Client, String> {
        let layer = GateLayer {
            gate: Arc::clone(gate),
            clock: Arc::clone(&self.clock),
            limit: limits.connect,
        };
        let mut builder = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .use_preconfigured_tls(self.tls.clone())
            .no_proxy()
            .pool_max_idle_per_host(0)
            .connector_layer(layer);
        if let Some(proxy) = &self.proxy {
            builder = builder.proxy(proxy_rule(proxy.clone()));
        }
        builder.build().map_err(|e| e.to_string())
    }

    fn request(
        &self,
        client: &reqwest::Client,
        request: &WireRequest,
    ) -> Result<reqwest::RequestBuilder, String> {
        let url = self
            .base
            .join(&request.path)
            .map_err(|e| format!("request path: {e}"))?;
        let mut builder = match request.method {
            WireMethod::Get => client.get(url),
            WireMethod::Post => client.post(url),
        };
        builder = builder
            .header(reqwest::header::CONNECTION, "close")
            .header(AUTHORIZATION, self.bearer.clone())
            .header(CLIENT_VERSION_HEADER, self.version.clone());
        if let Some(key) = &request.idempotency_key {
            builder = builder.header(IDEMPOTENCY_HEADER, key.as_str());
        }
        if let Some(body) = &request.body {
            builder = builder
                .header(CONTENT_TYPE, "application/json")
                .body(body.clone());
        }
        Ok(builder)
    }

    async fn run(
        &self,
        request: WireRequest,
        guard: &RequestGuard,
    ) -> Result<WireReply, WireFailure> {
        let gate = ConnectGate::new();
        let client = self
            .client(&gate, request.limits)
            .map_err(WireFailure::Network)?;
        let builder = self
            .request(&client, &request)
            .map_err(WireFailure::Network)?;
        let response = crate::http_phase::send(&self.clock, &gate, request.limits, builder).await?;
        guard.bytes_arrived();
        crate::http_phase::collect(&self.clock, request.limits, response, guard).await
    }
}

impl Wire for HttpWire {
    fn call<'a>(&'a self, request: WireRequest, guard: &'a RequestGuard) -> WireFuture<'a> {
        Box::pin(self.run(request, guard))
    }
}

fn proxy_rule(proxy: url::Url) -> reqwest::Proxy {
    reqwest::Proxy::custom(move |target| {
        if goes_direct(target) {
            None
        } else {
            Some(proxy.clone())
        }
    })
}
