use std::collections::HashMap;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::{Arc, Mutex};

use crate::types::{ProviderEndpoint, ProviderKind, WireFormat};
use crate::{wire_claude, wire_openai};

pub(crate) struct BuiltClient {
    pub client: reqwest::Client,
    pub os_notes: Vec<String>,
    pub pins: PinMap,
}

pub(crate) type PinMap = Arc<Mutex<HashMap<String, SocketAddr>>>;

const LOCAL_BYPASS: &str = "localhost,127.0.0.0/8,::1";

struct DialResolver {
    pins: PinMap,
}

pub(crate) fn product_client(
    trusted: &anybot_foundation::TrustedCerts,
    proxy: Option<&str>,
) -> Result<BuiltClient, String> {
    client_for(trusted, None, proxy)
}

pub(crate) fn client_for(
    trusted: &anybot_foundation::TrustedCerts,
    pin: Option<(&str, SocketAddr)>,
    proxy: Option<&str>,
) -> Result<BuiltClient, String> {
    let store = anybot_foundation::TrustStore::new(trusted)?;
    let notes = store.os_notes().to_vec();
    let tls = store.client_config()?;
    let pins = Arc::new(Mutex::new(HashMap::new()));
    if let Some((host, addr)) = pin {
        insert_pin(&pins, host, addr);
    }
    let mut builder = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .use_preconfigured_tls(tls)
        .no_proxy()
        .dns_resolver(Arc::new(DialResolver {
            pins: Arc::clone(&pins),
        }));
    if let Some(raw) = proxy {
        builder = builder.proxy(explicit_proxy(raw)?);
    }
    let client = builder.build().map_err(|err| err.to_string())?;
    Ok(BuiltClient {
        client,
        os_notes: notes,
        pins,
    })
}

impl reqwest::dns::Resolve for DialResolver {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let host = name.as_str().to_owned();
        let pinned = pin_of(&self.pins, &host);
        Box::pin(async move {
            let addrs: reqwest::dns::Addrs = if let Some(addr) = pinned {
                Box::new(std::iter::once(addr))
            } else {
                let looked = tokio::task::spawn_blocking(move || lookup_host(&host))
                    .await
                    .map_err(|err| -> Box<dyn std::error::Error + Send + Sync> { Box::new(err) })?
                    .map_err(|err| -> Box<dyn std::error::Error + Send + Sync> { Box::new(err) })?;
                Box::new(looked.into_iter())
            };
            Ok(addrs)
        })
    }
}

fn lookup_host(host: &str) -> Result<Vec<SocketAddr>, std::io::Error> {
    (host, 0).to_socket_addrs().map(Iterator::collect)
}

pub(crate) fn insert_pin(pins: &PinMap, host: &str, addr: SocketAddr) {
    match pins.lock() {
        Ok(mut rows) => {
            rows.insert(host.to_ascii_lowercase(), addr);
        }
        Err(poisoned) => {
            poisoned
                .into_inner()
                .insert(host.to_ascii_lowercase(), addr);
        }
    }
}

pub(crate) fn pin_of(pins: &PinMap, host: &str) -> Option<SocketAddr> {
    match pins.lock() {
        Ok(rows) => rows.get(&host.to_ascii_lowercase()).copied(),
        Err(poisoned) => poisoned
            .into_inner()
            .get(&host.to_ascii_lowercase())
            .copied(),
    }
}

fn explicit_proxy(raw: &str) -> Result<reqwest::Proxy, String> {
    let parsed = reqwest::Url::parse(raw).map_err(|err| format!("proxy url: {err}"))?;
    reqwest::Proxy::all(parsed)
        .map(|proxy| proxy.no_proxy(reqwest::NoProxy::from_string(LOCAL_BYPASS)))
        .map_err(|err| format!("proxy: {err}"))
}

pub(crate) fn chat_url(endpoint: &ProviderEndpoint) -> String {
    let path = match endpoint.format {
        WireFormat::Anthropic => wire_claude::MESSAGES_PATH,
        WireFormat::OpenAiCompatible => wire_openai::CHAT_PATH,
    };
    format!("{}{path}", endpoint.base_url)
}

pub(crate) fn needs_key(kind: ProviderKind) -> bool {
    !matches!(kind, ProviderKind::Ollama | ProviderKind::Custom)
}
