pub(crate) fn wants_stream(body: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return false;
    };
    value
        .get("stream")
        .is_some_and(|s| s.as_bool().unwrap_or(false))
}

pub(crate) fn requested_model(body: &str) -> String {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return String::new();
    };
    value
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or_default()
        .to_owned()
}
