use serde_json::Value;

pub(crate) fn insert(body: &mut Value, key: &str, value: Value) {
    if let Some(map) = body.as_object_mut() {
        map.insert(key.to_owned(), value);
    }
}

pub(crate) fn set_owned(slot: &mut String, value: &str) {
    slot.clear();
    slot.push_str(value);
}

pub(crate) fn u64_at(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}
