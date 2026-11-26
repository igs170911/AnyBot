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
    tls: Arc<rustls::ClientConfig>,
}

impl ReqwestWire {
    pub(crate) fn new(built: crate::client_http::BuiltClient) -> Arc<Self> {
        Arc::new(Self {
            client: built.client,
            pins: built.pins,
            tls: built.tls,
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
        if let Some(addr) = request.dial
            && let Ok(parsed) = reqwest::Url::parse(&request.url)
            && let Some(host) = parsed.host_str()
        {
            self.pin(host, addr);
        }
        let url = send_url(self, request);
        let mut builder = match request.method {
            crate::http_wire::WireMethod::Get => self.client.get(url),
            crate::http_wire::WireMethod::Post => self.client.post(url),
        };
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
            let target = dial_target(self, request).await?;
            let stream = tokio::net::TcpStream::connect(target)
                .await
                .map_err(|_| TransportFail::Unreachable)?;
            finish_connect(self, request, stream).await?;
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
        return Ok(SocketAddr::new(addr.ip(), addr.port()));
    }
    lookup_host(&host, port).await
}

async fn finish_connect(
    wire: &ReqwestWire,
    request: &WireRequest,
    stream: tokio::net::TcpStream,
) -> Result<(), TransportFail> {
    let parsed = reqwest::Url::parse(&request.url).map_err(|_| TransportFail::Unreachable)?;
    if parsed.scheme() != "https" {
        drop(stream);
        return Ok(());
    }
    let host = parsed
        .host_str()
        .ok_or(TransportFail::Unreachable)?
        .to_owned();
    let name = server_name(&host)?;
    let connector = tokio_rustls::TlsConnector::from(Arc::clone(&wire.tls));
    let _shaken = connector
        .connect(name, stream)
        .await
        .map_err(|err| map_handshake(&err))?;
    Ok(())
}

fn server_name(host: &str) -> Result<rustls::pki_types::ServerName<'static>, TransportFail> {
    rustls::pki_types::ServerName::try_from(host.to_owned()).map_err(|_| TransportFail::Unreachable)
}

fn map_handshake(err: &std::io::Error) -> TransportFail {
    if let Some(fault) = anybot_foundation::classify_tls_error(err) {
        return TransportFail::Certificate(fault);
    }
    TransportFail::Unreachable
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

fn send_url(wire: &ReqwestWire, request: &WireRequest) -> String {
    let Ok(mut parsed) = reqwest::Url::parse(&request.url) else {
        return request.url.clone();
    };
    let Some(host) = parsed.host_str().map(str::to_owned) else {
        return request.url.clone();
    };
    let Some(addr) = crate::client_http::pin_of(&wire.pins, &host) else {
        return request.url.clone();
    };
    if host.parse::<std::net::IpAddr>().is_ok() {
        let _ip = parsed.set_ip_host(addr.ip());
    }
    if parsed.set_port(Some(addr.port())).is_err() {
        return request.url.clone();
    }
    parsed.to_string()
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
