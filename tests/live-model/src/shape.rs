pub fn json_shape(body: &str) -> Result<Vec<String>, String> {
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("body is not json: {e}"))?;
    let mut out = vec![token_at(&value, "object")];
    out.extend(sorted_keys(value.as_object()));
    out.extend(choice_shape(&value));
    Ok(out)
}

fn choice_shape(value: &serde_json::Value) -> Vec<String> {
    let mut out = Vec::new();
    let Some(choices) = value.get("choices").and_then(|c| c.as_array()) else {
        out.push("choices:absent".to_owned());
        return out;
    };
    let Some(first) = choices.first() else {
        out.push("choices:empty".to_owned());
        return out;
    };
    let Some(reason) = first.get("finish_reason") else {
        out.push("finish_reason:absent".to_owned());
        return out;
    };
    out.push(format!(
        "finish_reason={}",
        reason.as_str().unwrap_or("non-string")
    ));
    if let Some(message) = first.get("message") {
        out.extend(sorted_keys(message.as_object()));
        out.extend(tool_call_shape(message));
        return out;
    }
    if let Some(delta) = first.get("delta") {
        out.extend(sorted_keys(delta.as_object()));
        out.extend(tool_call_shape(delta));
    }
    out
}

fn tool_call_shape(part: &serde_json::Value) -> Vec<String> {
    let mut out = Vec::new();
    let Some(calls) = part.get("tool_calls").and_then(|c| c.as_array()) else {
        return out;
    };
    out.push(format!("tool_calls:count={}", calls.len()));
    for call in calls {
        out.extend(sorted_keys(call.as_object()));
        out.extend(prefixed(call.as_object(), "tool"));
        if let Some(function) = call.get("function") {
            out.extend(sorted_keys(function.as_object()));
            out.extend(prefixed(function.as_object(), "function"));
        }
    }
    out
}

fn prefixed(map: Option<&serde_json::Map<String, serde_json::Value>>, prefix: &str) -> Vec<String> {
    let Some(entries) = map else {
        return Vec::new();
    };
    let mut keys: Vec<String> = entries.keys().map(|k| format!("{prefix}.{k}")).collect();
    keys.sort();
    keys
}

fn sorted_keys(map: Option<&serde_json::Map<String, serde_json::Value>>) -> Vec<String> {
    let Some(entries) = map else {
        return vec!["not-object".to_owned()];
    };
    let mut keys: Vec<String> = entries.keys().cloned().collect();
    keys.sort();
    keys
}

fn token_at(value: &serde_json::Value, key: &str) -> String {
    let found = value.get(key).and_then(|v| v.as_str());
    found.map_or_else(|| format!("{key}:absent"), |s| format!("{key}={s}"))
}

pub fn stream_shape(lines: &[String]) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for line in lines {
        let Some(payload) = data_payload(line) else {
            out.push(format!("line={line}"));
            continue;
        };
        if payload.trim() == CODE_DONE {
            out.push(format!("end={CODE_DONE}"));
            continue;
        }
        let value: serde_json::Value =
            serde_json::from_str(payload).map_err(|e| format!("chunk is not json: {e}"))?;
        out.push(token_at(&value, "object"));
        out.extend(sorted_keys(value.as_object()));
        out.extend(choice_shape(&value));
    }
    Ok(out)
}

pub const CODE_DONE: &str = "[DONE]";

fn data_payload(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    trimmed.strip_prefix("data:").map(str::trim)
}
