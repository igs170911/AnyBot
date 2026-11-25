use crate::types::{ProviderEndpoint, ProviderKind, WireFormat};
use crate::{wire_claude, wire_openai};

pub(crate) fn product_client(
    trusted: &anybot_foundation::TrustedCerts,
) -> Result<reqwest::Client, String> {
    client_for(trusted, None)
}

pub(crate) fn client_for(
    trusted: &anybot_foundation::TrustedCerts,
    pin: Option<(&str, std::net::SocketAddr)>,
) -> Result<reqwest::Client, String> {
    let store = anybot_foundation::TrustStore::new(trusted)?;
    let tls = store.client_config()?;
    let mut builder = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .use_preconfigured_tls(tls);
    if let Some((host, addr)) = pin {
        builder = builder.resolve(host, addr);
    }
    builder.build().map_err(|err| err.to_string())
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
