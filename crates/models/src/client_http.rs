use crate::types::{ProviderEndpoint, ProviderKind, WireFormat};
use crate::{wire_claude, wire_openai};

pub(crate) fn product_client() -> reqwest::Client {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
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
