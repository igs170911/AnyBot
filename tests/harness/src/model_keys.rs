use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum KeyMatch {
    Missing,
    Match,
    Mismatch,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct RecordedRequest {
    pub path: String,
    pub body: String,
    pub headers: Vec<(String, HeaderValueKind)>,
    pub key_match: KeyMatch,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HeaderValueKind {
    Present,
    Value(String),
}

const SENSITIVE_HEADERS: [&str; 6] = [
    "authorization",
    "x-api-key",
    "api-key",
    "x-goog-api-key",
    "openai-key",
    "anthropic-key",
];

pub fn record_headers(
    entries: impl Iterator<Item = (String, String)>,
) -> Vec<(String, HeaderValueKind)> {
    let mut recorded = Vec::new();
    for (name, value) in entries {
        let lowered = name.to_ascii_lowercase();
        if SENSITIVE_HEADERS.iter().any(|s| *s == lowered) {
            recorded.push((lowered, HeaderValueKind::Present));
            continue;
        }
        recorded.push((lowered, HeaderValueKind::Value(value)));
    }
    recorded.sort_by(|a, b| a.0.cmp(&b.0));
    recorded
}

#[must_use]
pub fn presented_key(name: &str, value: &str) -> Option<String> {
    let lowered = name.to_ascii_lowercase();
    if lowered == "authorization" {
        let rest = value
            .strip_prefix("Bearer ")
            .or_else(|| value.strip_prefix("bearer "))?;
        return Some(rest.to_owned());
    }
    if SENSITIVE_HEADERS.iter().any(|s| *s == lowered) {
        return Some(value.to_owned());
    }
    None
}

#[must_use]
pub fn classify_key(expected: Option<&str>, presented: Option<&str>) -> KeyMatch {
    match (expected, presented) {
        (_, None) => KeyMatch::Missing,
        (Some(want), Some(got)) if want == got => KeyMatch::Match,
        (None | Some(_), Some(_)) => KeyMatch::Mismatch,
    }
}

pub const CODE_MODEL_SCRIPT_EMPTY: &str = "model-script-empty";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StallDelay {
    pub span: Duration,
}
