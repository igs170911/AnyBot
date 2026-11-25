use serde_json::Value;

pub(crate) fn insert(body: &mut Value, key: &str, value: Value) {
    if let Some(map) = body.as_object_mut() {
        map.insert(key.to_owned(), value);
    }
}
