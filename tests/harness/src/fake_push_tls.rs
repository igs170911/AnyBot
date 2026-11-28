use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::Router;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

use crate::fake_push::{FakePush, push_router};
use crate::model_tls::{MintedCert, mint_self_signed, server_config};

const SERVE_DEADLINE: std::time::Duration = std::time::Duration::from_secs(600);

pub struct PushTls {
    pub push: Arc<FakePush>,
    pub port: u16,
    pub cert: MintedCert,
    pub landing_hits: Arc<AtomicUsize>,
}

pub fn push_client_builder(
    trusted_pem: &str,
    host: &str,
    port: u16,
) -> Result<reqwest::ClientBuilder, String> {
    let trusted = anybot_foundation::TrustedCerts::parse_pem(trusted_pem)?;
    let store = anybot_foundation::TrustStore::new(&trusted)?;
    let tls = store.client_config()?;
    let addr = format!("127.0.0.1:{port}")
        .parse()
        .map_err(|err| format!("test address: {err}"))?;
    Ok(reqwest::Client::builder()
        .use_preconfigured_tls(tls)
        .no_proxy()
        .resolve(host, addr))
}

pub async fn push_start_tls(dns: &str) -> Result<PushTls, String> {
    let cert = mint_self_signed(dns, 2)?;
    let config = server_config(&cert)?;
    let (std_listener, port) = crate::ports::bind_v4()?;
    std_listener
        .set_nonblocking(true)
        .map_err(|err| format!("set nonblocking failed: {err}"))?;
    let listener = TcpListener::from_std(std_listener)
        .map_err(|err| format!("convert listener failed: {err}"))?;
    let push = Arc::new(FakePush::default());
    let landing_hits = Arc::new(AtomicUsize::new(0));
    let router = push_router(Arc::clone(&push)).merge(redirect_router(Arc::clone(&landing_hits)));
    let incoming = TlsIncoming {
        listener,
        acceptor: TlsAcceptor::from(config),
    };
    tokio::spawn(async move {
        let _served = axum::serve(incoming, router)
            .with_graceful_shutdown(serve_deadline())
            .await;
    });
    Ok(PushTls {
        push,
        port,
        cert,
        landing_hits,
    })
}

fn redirect_router(hits: Arc<AtomicUsize>) -> Router {
    Router::new()
        .route("/redirect", any(redirect_elsewhere))
        .route("/landing", any(landing))
        .route("/health", get(|| async { StatusCode::OK }))
        .with_state(hits)
}

async fn redirect_elsewhere() -> Response {
    (StatusCode::FOUND, [(header::LOCATION, "/landing")]).into_response()
}

async fn landing(State(hits): State<Arc<AtomicUsize>>) -> Response {
    hits.fetch_add(1, Ordering::SeqCst);
    StatusCode::CREATED.into_response()
}

async fn serve_deadline() {
    tokio::time::sleep(SERVE_DEADLINE).await;
}

struct TlsIncoming {
    listener: TcpListener,
    acceptor: TlsAcceptor,
}

impl axum::serve::Listener for TlsIncoming {
    type Io = tokio_rustls::server::TlsStream<tokio::net::TcpStream>;
    type Addr = std::net::SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        loop {
            let Ok((stream, peer)) = self.listener.accept().await else {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                continue;
            };
            if let Ok(tls) = self.acceptor.accept(stream).await {
                return (tls, peer);
            }
        }
    }

    fn local_addr(&self) -> std::io::Result<Self::Addr> {
        self.listener.local_addr()
    }
}
