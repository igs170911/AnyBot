use anybot_foundation::ContentDigest;

#[must_use]
pub fn append_key(stream: &str, record_ids: &[&str]) -> String {
    let mut joined = String::from(stream);
    for id in record_ids {
        joined.push('\n');
        joined.push_str(id);
    }
    format!("append-{}", ContentDigest::sha256(joined.as_bytes()).hex())
}

#[must_use]
pub fn action_key(action_id: &str) -> String {
    format!("action-{action_id}")
}
