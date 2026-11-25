use crate::types::ProviderKind;

#[must_use]
pub fn context_limit(message: &str) -> Option<u64> {
    let digits: String = message
        .chars()
        .map(|ch| if ch.is_ascii_digit() { ch } else { ' ' })
        .collect();
    digits
        .split_whitespace()
        .filter_map(|part| part.parse::<u64>().ok())
        .max()
}

#[must_use]
pub fn provider_label(kind: ProviderKind) -> &'static str {
    match kind {
        ProviderKind::Xai => "xAI",
        ProviderKind::OpenAi => "OpenAI",
        ProviderKind::Anthropic => "Claude",
        ProviderKind::Gemini => "Gemini",
        ProviderKind::OpenRouter => "OpenRouter",
        ProviderKind::OpenCodeGo => "OpenCode Go",
        ProviderKind::Ollama => "Ollama",
        ProviderKind::Custom => "Custom",
    }
}

#[must_use]
pub fn redact(text: &str, secret: Option<&str>) -> String {
    let Some(secret) = secret.filter(|value| !value.is_empty()) else {
        return text.to_owned();
    };
    text.replace(secret, "[redacted]")
}

#[must_use]
pub fn clip_detail(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return "unknown".to_owned();
    }
    let mut out = String::new();
    for ch in trimmed.chars().take(300) {
        out.push(ch);
    }
    out
}

#[must_use]
pub(crate) fn too_long(message: &str) -> bool {
    let lowered = message.to_ascii_lowercase();
    lowered.contains("context length")
        || lowered.contains("maximum context")
        || lowered.contains("too long")
        || lowered.contains("prompt is too long")
        || lowered.contains("context_length")
}

#[must_use]
pub(crate) fn no_tools(message: &str) -> bool {
    let lowered = message.to_ascii_lowercase();
    lowered.contains("does not support tools")
        || lowered.contains("tool use is not supported")
        || lowered.contains("unsupported tool")
}

#[must_use]
pub(crate) fn rejected_param(message: &str) -> bool {
    let lowered = message.to_ascii_lowercase();
    lowered.contains("unsupported parameter")
        || lowered.contains("unknown parameter")
        || lowered.contains("does not support reasoning")
        || lowered.contains("unexpected argument")
}

#[must_use]
pub(crate) fn quota(status: u16, message: &str) -> bool {
    if status == 402 {
        return true;
    }
    let lowered = message.to_ascii_lowercase();
    lowered.contains("insufficient_quota") || lowered.contains("usage limit")
}
