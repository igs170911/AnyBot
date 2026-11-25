use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GapRecord {
    pub failing_item: String,
    pub reason: String,
}

impl GapRecord {
    #[must_use]
    pub fn new(failing_item: &str, reason: &str) -> Self {
        GapRecord {
            failing_item: failing_item.to_owned(),
            reason: reason.to_owned(),
        }
    }
}

#[must_use]
pub fn same_gap(prev: &GapRecord, next: &GapRecord) -> bool {
    prev.failing_item == next.failing_item
        && numbers_stable(&prev.reason, &next.reason)
        && reason_similarity(&prev.reason, &next.reason) >= SIMILARITY_FLOOR
}

const SIMILARITY_FLOOR: f64 = 0.7;

#[must_use]
pub fn numbers_stable(prev: &str, next: &str) -> bool {
    digits(prev) == digits(next)
}

fn digits(text: &str) -> Vec<String> {
    let Ok(re) = Regex::new(r"\d+") else {
        return Vec::new();
    };
    re.find_iter(text).map(|m| m.as_str().to_owned()).collect()
}

fn normalize(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn bigrams(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() < 2 {
        return chars.iter().map(ToString::to_string).collect();
    }
    chars.windows(2).map(|w| w.iter().collect()).collect()
}

#[must_use]
pub fn reason_similarity(prev: &str, next: &str) -> f64 {
    let a = counts(&bigrams(&normalize(prev)));
    let b = counts(&bigrams(&normalize(next)));
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let mut common = 0usize;
    let mut union = 0usize;
    for key in a.keys().chain(b.keys()) {
        let left = a.get(key).copied().unwrap_or(0);
        let right = b.get(key).copied().unwrap_or(0);
        common = common.saturating_add(left.min(right));
        union = union.saturating_add(left.max(right));
    }
    if union == 0 {
        return 1.0;
    }
    f64::from(u32::try_from(common).unwrap_or(u32::MAX))
        / f64::from(u32::try_from(union).unwrap_or(u32::MAX))
}

fn counts(items: &[String]) -> std::collections::BTreeMap<String, usize> {
    let mut map = std::collections::BTreeMap::new();
    for item in items {
        let next = map.get(item).copied().unwrap_or(0usize).saturating_add(1);
        map.insert(item.clone(), next);
    }
    map
}

#[cfg(test)]
#[path = "goal_brake_tests.rs"]
mod tests;
