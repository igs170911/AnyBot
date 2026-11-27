use serde_json::Value;

pub const KIND_RECORD_DELETE: &str = "record_delete";
pub const KIND_RECORD_CLEAR: &str = "record_purge";
pub const KIND_RECORD_RESTORE: &str = "record_restore";
pub const KIND_RECORD_PURGE_PERMANENT: &str = "record_purge_permanent";

pub const RESTORE_WINDOW_MS: i64 = 30 * 24 * 60 * 60 * 1000;

#[must_use]
pub fn is_chat_kind(kind: &str) -> bool {
    matches!(kind, "message" | "model_answer")
}

#[must_use]
pub fn is_hideable_action_kind(kind: &str) -> bool {
    matches!(kind, KIND_RECORD_DELETE | KIND_RECORD_CLEAR)
}

#[must_use]
pub fn restore_window_open(action_at_ms: i64, now_ms: i64) -> Option<bool> {
    now_ms
        .checked_sub(action_at_ms)
        .map(|age| age <= RESTORE_WINDOW_MS)
}

#[must_use]
pub fn scrub_chat_content(kind: &str, content: &Value) -> Value {
    match content {
        Value::Object(map) if kind == "model_answer" => {
            let mut scrubbed = map.clone();
            for key in ["text", "thinking"] {
                if let Some(slot) = scrubbed.get_mut(key) {
                    *slot = Value::Null;
                }
            }
            Value::Object(scrubbed)
        }
        Value::Null
        | Value::Bool(_)
        | Value::Number(_)
        | Value::String(_)
        | Value::Array(_)
        | Value::Object(_) => Value::Null,
    }
}
