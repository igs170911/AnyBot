use serde_json::Value;

pub trait SecretMasker: Send + Sync {
    fn mask_secrets(&self, input: &str) -> String;
}

pub struct BasicSecretMasker;

impl SecretMasker for BasicSecretMasker {
    fn mask_secrets(&self, input: &str) -> String {
        mask_text(input)
    }
}

pub(crate) fn mask_json(value: Value, masker: &dyn SecretMasker) -> Value {
    match value {
        Value::String(text) => Value::String(masker.mask_secrets(&text)),
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .map(|item| mask_json(item, masker))
                .collect(),
        ),
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(k, v)| (k, mask_json(v, masker)))
                .collect(),
        ),
        Value::Null | Value::Bool(_) | Value::Number(_) => value,
    }
}

fn mask_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while !rest.is_empty() {
        if let Some((prefix, after)) = match_auth_prefix(rest) {
            out.push_str(prefix);
            out.push_str("[REDACTED]");
            rest = skip_token(after);
        } else if let Some(ch) = rest.chars().next() {
            out.push(ch);
            rest = rest.strip_prefix(ch).unwrap_or("");
        } else {
            break;
        }
    }
    out
}

fn match_auth_prefix(text: &str) -> Option<(&'static str, &str)> {
    const PREFIXES: [&str; 6] = [
        "Bearer ", "bearer ", "Basic ", "api_key=", "token=", "secret=",
    ];
    for prefix in PREFIXES {
        if let Some(after) = text.strip_prefix(prefix) {
            return Some((prefix, after));
        }
    }
    None
}

fn skip_token(text: &str) -> &str {
    match text.find(|ch: char| ch.is_whitespace() || matches!(ch, '"' | '\'' | ',' | ')' | ']')) {
        Some(idx) => text.split_at_checked(idx).map_or("", |(_, tail)| tail),
        None => "",
    }
}
