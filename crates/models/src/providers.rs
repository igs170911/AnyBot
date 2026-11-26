use anybot_foundation::Secret;

use crate::types::{ProviderEndpoint, ProviderKind, WireFormat};

pub const CLAUDE_MAX_OUTPUT: u64 = 64_000;
pub const OPENROUTER_APP: &str = "AnyBot";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProviderDefaults {
    pub kind: ProviderKind,
    pub default_base: &'static str,
    pub suggested_model: &'static str,
    pub format: WireFormat,
    pub needs_key: bool,
}

pub const PROVIDERS: [ProviderDefaults; 8] = [
    ProviderDefaults {
        kind: ProviderKind::Xai,
        default_base: "https://api.x.ai/v1",
        suggested_model: "grok-4",
        format: WireFormat::OpenAiCompatible,
        needs_key: true,
    },
    ProviderDefaults {
        kind: ProviderKind::OpenAi,
        default_base: "https://api.openai.com/v1",
        suggested_model: "gpt-5",
        format: WireFormat::OpenAiCompatible,
        needs_key: true,
    },
    ProviderDefaults {
        kind: ProviderKind::Anthropic,
        default_base: "https://api.anthropic.com/v1",
        suggested_model: "claude-opus-5-5",
        format: WireFormat::Anthropic,
        needs_key: true,
    },
    ProviderDefaults {
        kind: ProviderKind::Gemini,
        default_base: "https://generativelanguage.googleapis.com/v1beta/openai",
        suggested_model: "gemini-2.5-flash",
        format: WireFormat::OpenAiCompatible,
        needs_key: true,
    },
    ProviderDefaults {
        kind: ProviderKind::OpenRouter,
        default_base: "https://openrouter.ai/api/v1",
        suggested_model: "openrouter/auto",
        format: WireFormat::OpenAiCompatible,
        needs_key: true,
    },
    ProviderDefaults {
        kind: ProviderKind::OpenCodeGo,
        default_base: "https://opencode.ai/inference/go",
        suggested_model: "kimi-k3",
        format: WireFormat::OpenAiCompatible,
        needs_key: true,
    },
    ProviderDefaults {
        kind: ProviderKind::Ollama,
        default_base: "http://localhost:11434/v1",
        suggested_model: "llama3.2",
        format: WireFormat::OpenAiCompatible,
        needs_key: false,
    },
    ProviderDefaults {
        kind: ProviderKind::Custom,
        default_base: "http://localhost:8080/v1",
        suggested_model: "local-model",
        format: WireFormat::OpenAiCompatible,
        needs_key: false,
    },
];

#[must_use]
pub fn defaults_for(kind: ProviderKind) -> ProviderDefaults {
    match kind {
        ProviderKind::Xai => PROVIDERS[0],
        ProviderKind::OpenAi => PROVIDERS[1],
        ProviderKind::Anthropic => PROVIDERS[2],
        ProviderKind::Gemini => PROVIDERS[3],
        ProviderKind::OpenRouter => PROVIDERS[4],
        ProviderKind::OpenCodeGo => PROVIDERS[5],
        ProviderKind::Ollama => PROVIDERS[6],
        ProviderKind::Custom => PROVIDERS[7],
    }
}

#[must_use]
pub fn trim_base(raw: &str, fallback: &str) -> String {
    let chosen = if raw.trim().is_empty() {
        fallback
    } else {
        raw.trim()
    };
    chosen.trim_end_matches('/').to_owned()
}

#[must_use]
pub fn trim_key(raw: &str) -> Option<Secret> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(Secret::from_untrusted(trimmed.to_owned()))
}

#[must_use]
pub fn resolve_endpoint(
    kind: ProviderKind,
    base_url: &str,
    api_key: &str,
    model: &str,
    format: Option<WireFormat>,
) -> ProviderEndpoint {
    let defaults = defaults_for(kind);
    let resolved_model = if model.trim().is_empty() {
        defaults.suggested_model.to_owned()
    } else {
        model.trim().to_owned()
    };
    ProviderEndpoint {
        kind,
        base_url: trim_base(base_url, defaults.default_base),
        api_key: trim_key(api_key),
        model: resolved_model,
        format: format.unwrap_or(defaults.format),
    }
}

#[must_use]
pub fn key_allowed(endpoint: &ProviderEndpoint, destination: &str) -> bool {
    let Ok(base) = url::Url::parse(&endpoint.base_url) else {
        return false;
    };
    let Ok(target) = url::Url::parse(destination) else {
        return false;
    };
    hosts_match(&base, &target) && under_base(&base, &target)
}

fn hosts_match(base: &url::Url, target: &url::Url) -> bool {
    let Some(registered) = base.host_str() else {
        return false;
    };
    let Some(dialed) = target.host_str() else {
        return false;
    };
    registered.eq_ignore_ascii_case(dialed) && port_matches(base, target)
}

fn port_matches(base: &url::Url, target: &url::Url) -> bool {
    base.port_or_known_default() == target.port_or_known_default()
}

fn under_base(base: &url::Url, target: &url::Url) -> bool {
    let root = base.path().trim_end_matches('/');
    let path = target.path();
    path == root || path.starts_with(&format!("{root}/"))
}

#[must_use]
pub fn opencode_format(model: &str) -> WireFormat {
    if model.starts_with("claude") {
        WireFormat::Anthropic
    } else {
        WireFormat::OpenAiCompatible
    }
}
