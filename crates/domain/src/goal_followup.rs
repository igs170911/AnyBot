use regex::Regex;

#[must_use]
pub fn is_slash_wrapped(pattern: &str) -> bool {
    pattern.len() > 2 && pattern.starts_with('/') && pattern.ends_with('/')
}

#[must_use]
pub fn matches_done_when(pattern: &str, output: &str) -> bool {
    if !is_slash_wrapped(pattern) {
        return output.contains(pattern);
    }
    let Some(inner) = pattern.strip_prefix('/').and_then(|t| t.strip_suffix('/')) else {
        return output.contains(pattern);
    };
    match Regex::new(inner) {
        Ok(re) => re.is_match(output),
        Err(_) => output.contains(pattern),
    }
}

#[cfg(test)]
#[path = "goal_followup_tests.rs"]
mod tests;
