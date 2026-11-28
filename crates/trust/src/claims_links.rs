use crate::claims_text::strip_code_spans;

#[must_use]
pub(crate) fn has_dead_bot_link(reply: &str, files_placed: &[String]) -> bool {
    let plain = strip_code_spans(reply);
    for target in extract_targets(&plain) {
        if is_ignored_link(&target) {
            continue;
        }
        if is_bot_local_target(&target) {
            let name = target_basename(&target);
            if !has_matching_file(&name, files_placed) {
                return true;
            }
        }
    }
    false
}

fn extract_targets(text: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let mut idx = 0usize;
    while idx < text.len() {
        let Some(rest) = text.get(idx..) else {
            break;
        };
        if let Some(after) = rest.strip_prefix("](")
            && let Some(end) = after.find(')')
        {
            let target = after.get(..end).unwrap_or("").trim();
            if !target.is_empty() {
                targets.push(target.to_owned());
            }
            idx = idx.saturating_add(2).saturating_add(end).saturating_add(1);
            continue;
        }
        let Some(ch) = rest.chars().next() else {
            break;
        };
        idx = idx.saturating_add(ch.len_utf8());
    }
    for token in text.split_whitespace() {
        let cleaned = token.trim_matches(|c: char| matches!(c, '(' | ')' | '[' | ']' | '<' | '>'));
        if is_bot_local_target(cleaned) {
            targets.push(cleaned.to_owned());
        }
    }
    targets
}

fn is_ignored_link(target: &str) -> bool {
    let lower = target.to_lowercase();
    lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("mailto:")
        || lower.starts_with('#')
}

fn is_bot_local_target(target: &str) -> bool {
    let lower = target.to_lowercase();
    lower.starts_with("sandbox:")
        || lower.starts_with("file://")
        || target.starts_with("/home/")
        || target.starts_with("/Users/")
        || target.starts_with("~/")
        || target.starts_with("./")
        || has_windows_drive(target)
}

fn has_windows_drive(target: &str) -> bool {
    let mut chars = target.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    let Some(second) = chars.next() else {
        return false;
    };
    let Some(third) = chars.next() else {
        return false;
    };
    first.is_ascii_alphabetic() && second == ':' && (third == '\\' || third == '/')
}

fn target_basename(target: &str) -> String {
    let no_query = target.split(['?', '#']).next().unwrap_or(target);
    let trimmed = no_query.trim_end_matches(['/', '\\', '.', ',', ';']);
    trimmed
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("")
        .to_lowercase()
}

fn has_matching_file(basename_lower: &str, files_placed: &[String]) -> bool {
    if basename_lower.is_empty() {
        return false;
    }
    files_placed
        .iter()
        .any(|placed| target_basename(placed) == basename_lower)
}
