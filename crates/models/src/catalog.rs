pub const LIST_WAIT_SECS: u64 = 45;

#[must_use]
pub fn normalize_ids(raw: &[String]) -> Vec<String> {
    let mut names: Vec<String> = raw
        .iter()
        .map(|id| id.trim().trim_start_matches("models/").to_owned())
        .filter(|id| !id.is_empty())
        .collect();
    names.sort();
    names.dedup();
    names
}

#[must_use]
pub fn empty_list_code() -> &'static str {
    "model-list-empty"
}

#[must_use]
pub fn list_timeout_code() -> &'static str {
    "model-list-timeout"
}
