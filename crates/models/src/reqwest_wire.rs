use std::future::Future;
use std::net::{SocketAddr, ToSocketAddrs};
use std::pin::Pin;
use std::sync::Arc;

use futures_util::StreamExt;

use crate::http_wire::{Connected, HttpWire, WireBody, WireOpen, WireRequest};
use crate::transport::TransportFail;

pub(crate) struct ReqwestWire {
    client: reqwest::Client,
    pins: crate::client_http::PinMap,
}

impl ReqwestWire {
    pub(crate) fn new(built: crate::client_http::BuiltClient) -> Arc<Self> {
        Arc::new(Self {
            client: built.client,
            pins: built.pins,
        })
    }

    pub(crate) fn pin(&self, host: &str, addr: SocketAddr) {
        match self.pins.lock() {
            Ok(mut pins) => {
                pins.insert(host.to_ascii_lowercase(), addr);
            }
            Err(poisoned) => {
                poisoned
                    .into_inner()
                    .insert(host.to_ascii_lowercase(), addr);
            }
        }
    }

    fn builder(&self, request: &WireRequest) -> reqwest::RequestBuilder {
        let mut builder = match request.method {
            crate::http_wire::WireMethod::Get => self.client.get(&request.url),
            crate::http_wire::WireMethod::Post => self.client.post(&request.url),
        };
        if let Some(addr) = request.dial
            && let Ok(parsed) = reqwest::Url::parse(&request.url)
            && let Some(host) = parsed.host_str()
        {
            self.pin(host, addr);
        }
        for header in &request.headers {
            builder = builder.header(&header.name, header.value.expose());
        }
        if request.body.is_empty() {
            return builder;
        }
        builder
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(request.body.clone())
    }
}

impl HttpWire for ReqwestWire {
    fn connect<'a>(
        &'a self,
        request: &'a WireRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Connected, TransportFail>> + Send + 'a>> {
        Box::pin(async move {
            if request.url.starts_with("https://") {
                return Ok(Connected::Ready);
            }
            let target = dial_target(self, request).await?;
            let _stream = tokio::net::TcpStream::connect(target)
                .await
                .map_err(|_| TransportFail::Unreachable)?;
            Ok(Connected::Ready)
        })
    }

    fn open<'a>(
        &'a self,
        request: &'a WireRequest,
    ) -> Pin<Box<dyn Future<Output = Result<WireOpen, TransportFail>> + Send + 'a>> {
        Box::pin(async move {
            let response = self.builder(request).send().await.map_err(map_send)?;
            Ok(open_from(response))
        })
    }
}

fn open_from(response: reqwest::Response) -> WireOpen {
    let status = response.status().as_u16();
    let headers = header_pairs(response.headers());
    let stream = response
        .bytes_stream()
        .map(|item| item.map_err(|_| TransportFail::Dropped));
    WireOpen {
        status,
        headers,
        body: WireBody::Stream(Box::pin(stream)),
    }
}

async fn dial_target(
    wire: &ReqwestWire,
    request: &WireRequest,
) -> Result<SocketAddr, TransportFail> {
    let parsed = reqwest::Url::parse(&request.url).map_err(|_| TransportFail::Unreachable)?;
    let host = parsed
        .host_str()
        .ok_or(TransportFail::Unreachable)?
        .to_owned();
    let port = parsed
        .port_or_known_default()
        .ok_or(TransportFail::Unreachable)?;
    if let Some(addr) = request.dial {
        wire.pin(&host, addr);
    }
    if let Some(addr) = crate::client_http::pin_of(&wire.pins, &host) {
        return Ok(SocketAddr::new(addr.ip(), nonzero_port(addr.port(), port)));
    }
    lookup_host(&host, port).await
}

fn nonzero_port(pinned: u16, url_port: u16) -> u16 {
    if pinned == 0 { url_port } else { pinned }
}

async fn lookup_host(host: &str, port: u16) -> Result<SocketAddr, TransportFail> {
    let looked = tokio::task::spawn_blocking({
        let owned = host.to_owned();
        move || {
            format!("{owned}:{port}")
                .to_socket_addrs()
                .map(Iterator::collect::<Vec<_>>)
        }
    })
    .await
    .map_err(|_| TransportFail::Unreachable)?
    .map_err(|_| TransportFail::Unreachable)?;
    looked.into_iter().next().ok_or(TransportFail::Unreachable)
}

fn map_send(err: reqwest::Error) -> TransportFail {
    let mapped = crate::tls_fail::map_reqwest(&err);
    let connect = err.is_connect() || err.is_timeout();
    drop(err);
    if matches!(mapped, TransportFail::Certificate(_)) {
        return mapped;
    }
    if connect {
        return TransportFail::ConnectTimeout;
    }
    mapped
}

fn header_pairs(headers: &reqwest::header::HeaderMap) -> Vec<(String, String)> {
    headers
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|text| (name.as_str().to_owned(), text.to_owned()))
        })
        .collect()
}
