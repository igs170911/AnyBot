use crate::types::{ProviderEndpoint, ProviderKind, WireFormat};
use crate::{wire_claude, wire_openai};

pub(crate) struct BuiltClient {
    pub client: reqwest::Client,
    pub os_notes: Vec<String>,
}

pub(crate) fn product_client(
    trusted: &anybot_foundation::TrustedCerts,
    proxy: Option<&str>,
) -> Result<BuiltClient, String> {
    client_for(trusted, None, proxy)
}

pub(crate) fn client_for(
    trusted: &anybot_foundation::TrustedCerts,
    pin: Option<(&str, std::net::SocketAddr)>,
    proxy: Option<&str>,
) -> Result<BuiltClient, String> {
    let store = anybot_foundation::TrustStore::new(trusted)?;
    let notes = store.os_notes().to_vec();
    let tls = store.client_config()?;
    let mut builder = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .use_preconfigured_tls(tls)
        .no_proxy();
    if let Some(raw) = proxy {
        builder = builder.proxy(explicit_proxy(raw)?);
    }
    if let Some((host, addr)) = pin {
        builder = builder.resolve(host, addr);
    }
    let client = builder.build().map_err(|err| err.to_string())?;
    Ok(BuiltClient {
        client,
        os_notes: notes,
    })
}

fn explicit_proxy(raw: &str) -> Result<reqwest::Proxy, String> {
    let parsed = reqwest::Url::parse(raw).map_err(|err| format!("proxy url: {err}"))?;
    reqwest::Proxy::all(parsed)
        .map(|proxy| proxy.no_proxy(reqwest::NoProxy::from_string("localhost,127.0.0.1,::1")))
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
