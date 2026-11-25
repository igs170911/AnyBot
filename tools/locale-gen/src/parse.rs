use std::collections::BTreeMap;

const MAX_LINE_BYTES: usize = 400;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FtlEntry {
    pub id: String,
    pub params: Vec<String>,
}

#[must_use]
pub fn is_code_shape(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !text.starts_with('-')
        && !text.ends_with('-')
        && !text.contains("--")
}

fn collect_params(value: &str) -> Vec<String> {
    let mut found = Vec::new();
    let chars: Vec<char> = value.chars().collect();
    let mut i = 0;
    while let Some(ch) = chars.get(i) {
        if *ch == '$' {
            let mut name = String::new();
            let Some(mut j) = i.checked_add(1) else {
                break;
            };
            let starts = chars
                .get(j)
                .is_some_and(|c| c.is_ascii_alphabetic() || *c == '_');
            if starts {
                while let Some(inner) = chars.get(j) {
                    if !(inner.is_ascii_alphanumeric() || *inner == '_') {
                        break;
                    }
                    name.push(*inner);
                    j = match j.checked_add(1) {
                        Some(next) => next,
                        None => break,
                    };
                }
            }
            if !name.is_empty() && !found.contains(&name) {
                found.push(name);
            }
            i = j;
        } else {
            i = match i.checked_add(1) {
                Some(next) => next,
                None => break,
            };
        }
    }
    found.sort();
    found
}

fn entry_line(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim_end();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let (id, value) = trimmed.split_once(" = ")?;
    let id = id.trim();
    if !is_code_shape(id) {
        return None;
    }
    Some((id.to_owned(), value.to_owned()))
}

#[must_use]
pub fn parse_ftl(text: &str) -> Vec<FtlEntry> {
    let mut entries = Vec::new();
    let mut current: Option<(String, String)> = None;
    for line in text.lines() {
        if line.len() > MAX_LINE_BYTES {
            continue;
        }
        if let Some((id, value)) = entry_line(line) {
            if let Some((prev_id, prev_value)) = current.take() {
                entries.push(finish_entry(prev_id, &prev_value));
            }
            current = Some((id, value));
        } else if let Some((_, value)) = current.as_mut()
            && line.starts_with([' ', '\t'])
        {
            value.push('\n');
            value.push_str(line.trim_start());
        }
    }
    if let Some((id, value)) = current {
        entries.push(finish_entry(id, &value));
    }
    entries
}

fn finish_entry(id: String, value: &str) -> FtlEntry {
    FtlEntry {
        id,
        params: collect_params(value),
    }
}

#[must_use]
pub fn entries_map(entries: &[FtlEntry]) -> BTreeMap<String, Vec<String>> {
    entries
        .iter()
        .map(|e| (e.id.clone(), e.params.clone()))
        .collect()
}

#[must_use]
pub fn duplicate_ids(entries: &[FtlEntry]) -> Vec<String> {
    let mut seen = Vec::new();
    let mut dups = Vec::new();
    for entry in entries {
        if seen.contains(&entry.id) && !dups.contains(&entry.id) {
            dups.push(entry.id.clone());
        }
        seen.push(entry.id.clone());
    }
    dups
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ftl_extracts_ids_and_params() {
        let text = "# comment\n### section\nserver-x = Value { $port } and { $db_host }\nplain = No params\n";
        let entries = parse_ftl(text);
        let map = entries_map(&entries);
        assert_eq!(
            map.get("server-x").map(Vec::as_slice),
            Some(&["db_host".to_owned(), "port".to_owned()][..])
        );
        assert_eq!(map.get("plain").map(Vec::as_slice), Some(&[][..]));
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn test_parse_ftl_multiline_value_keeps_params() {
        let text = "multi = First line\n  Second { $count } line\nnext = Other\n";
        let map = entries_map(&parse_ftl(text));
        assert_eq!(
            map.get("multi").map(Vec::as_slice),
            Some(&["count".to_owned()][..])
        );
        assert!(map.contains_key("next"));
    }

    #[test]
    fn test_parse_ftl_flags_duplicate_ids() {
        let text = "dup = One\ndup = Two\n";
        assert_eq!(duplicate_ids(&parse_ftl(text)), vec!["dup".to_owned()]);
    }

    #[test]
    fn test_code_shape_accepts_kebab_and_rejects_bad_forms() {
        assert!(is_code_shape("server-startup-listening"));
        assert!(is_code_shape("ui-shell-skeleton2"));
        assert!(!is_code_shape("Server-Code"));
        assert!(!is_code_shape("-lead"));
        assert!(!is_code_shape("trail-"));
        assert!(!is_code_shape("double--dash"));
        assert!(!is_code_shape("with space"));
        assert!(!is_code_shape(""));
    }
}
