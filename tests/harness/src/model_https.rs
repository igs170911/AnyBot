use std::sync::Arc;

use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

use crate::fake_model::{FakeModel, ModelFormat, model_router};
use crate::model_tls::{MintedCert, mint_self_signed, server_config};

pub struct HttpsModel {
    pub model: Arc<FakeModel>,
    pub port: u16,
    pub cert: MintedCert,
}

pub fn https_client(trusted_pem: &str, host: &str, port: u16) -> Result<reqwest::Client, String> {
    let trusted = if trusted_pem.is_empty() {
        anybot_foundation::TrustedCerts::empty()
    } else {
        anybot_foundation::TrustedCerts::parse_pem(trusted_pem)?
    };
    let store = anybot_foundation::TrustStore::new(&trusted)?;
    let tls = store.client_config()?;
    let addr = format!("127.0.0.1:{port}")
        .parse()
        .map_err(|err| format!("test address: {err}"))?;
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .use_preconfigured_tls(tls)
        .no_proxy()
        .resolve(host, addr)
        .build()
        .map_err(|err| err.to_string())
}

pub async fn model_start_https(dns: &str) -> Result<HttpsModel, String> {
    let cert = mint_self_signed(dns, 2)?;
    model_start_with_cert(cert).await
}

pub async fn model_start_with_cert(cert: MintedCert) -> Result<HttpsModel, String> {
    let model = Arc::new(FakeModel::default());
    model.format_set(ModelFormat::OpenAi);
    let config = server_config(&cert)?;
    let (std_listener, port) = crate::ports::bind_v4()?;
    std_listener
        .set_nonblocking(true)
        .map_err(|err| format!("set nonblocking failed: {err}"))?;
    let listener = TcpListener::from_std(std_listener)
        .map_err(|err| format!("convert listener failed: {err}"))?;
    let acceptor = TlsAcceptor::from(config);
    let router = model_router(Arc::clone(&model));
    let incoming = TlsIncoming { listener, acceptor };
    tokio::spawn(async move {
        let _served = axum::serve(incoming, router)
            .with_graceful_shutdown(accept_deadline())
            .await;
    });
    Ok(HttpsModel { model, port, cert })
}

struct TlsIncoming {
    listener: TcpListener,
    acceptor: TlsAcceptor,
}

const ACCEPT_DEADLINE: std::time::Duration = std::time::Duration::from_secs(30);

async fn accept_deadline() {
    tokio::time::sleep(ACCEPT_DEADLINE).await;
}

async fn parked() -> (
    tokio_rustls::server::TlsStream<tokio::net::TcpStream>,
    std::net::SocketAddr,
) {
    std::future::pending().await
}

impl axum::serve::Listener for TlsIncoming {
    type Io = tokio_rustls::server::TlsStream<tokio::net::TcpStream>;
    type Addr = std::net::SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        loop {
            let Ok(Ok(accepted)) =
                tokio::time::timeout(ACCEPT_DEADLINE, self.listener.accept()).await
            else {
                return parked().await;
            };
            if let Ok(tls) = self.acceptor.accept(accepted.0).await {
                return (tls, accepted.1);
            }
        }
    }

    fn local_addr(&self) -> std::io::Result<Self::Addr> {
        self.listener.local_addr()
    }
}
