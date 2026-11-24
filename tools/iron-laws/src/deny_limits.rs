use crate::table::{ALLOWED_DENY_IGNORE, DENY_EXCEPTION_KEYS};

struct Section {
    name: String,
    entries: Vec<(String, String)>,
}

fn clean_line(raw: &str) -> String {
    match raw.split('#').next() {
        Some(x) => x.trim().to_owned(),
        None => String::new(),
    }
}

fn push_header(line: &str, current: &mut String, out: &mut Vec<Section>) {
    let Some(inner) = line.strip_prefix('[').and_then(|x| x.strip_suffix(']')) else {
        return;
    };
    current.clone_from(
        &inner
            .split(',')
            .next()
            .map_or(String::new(), |x| x.trim().trim_matches('"').to_owned()),
    );
    if !out.iter().any(|s: &Section| s.name == *current) {
        out.push(Section {
            name: current.clone(),
            entries: Vec::new(),
        });
    }
}

fn feed_pending(
    key: &str,
    acc: &str,
    line: &str,
    current: &str,
    out: &mut [Section],
    pending: &mut Option<(String, String)>,
) {
    if line.ends_with(']') {
        let joined = format!("{acc}{line}");
        if let Some(s) = out.iter_mut().find(|s| s.name == current) {
            s.entries.push((key.to_owned(), joined));
        }
    } else {
        *pending = Some((key.to_owned(), format!("{acc}{line}")));
    }
}

fn feed_pair(
    key: &str,
    value: &str,
    current: &str,
    out: &mut [Section],
    pending: &mut Option<(String, String)>,
) {
    if value.starts_with('[') && !value.ends_with(']') {
        *pending = Some((key.to_owned(), value.to_owned()));
        return;
    }
    if let Some(s) = out.iter_mut().find(|s| s.name == current) {
        s.entries.push((key.to_owned(), value.to_owned()));
    }
}

fn parse_sections(content: &str) -> Vec<Section> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut pending: Option<(String, String)> = None;
    for raw in content.lines() {
        let line = clean_line(raw);
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            push_header(&line, &mut current, &mut out);
            continue;
        }
        if let Some((key, acc)) = pending.take() {
            feed_pending(&key, &acc, &line, &current, &mut out, &mut pending);
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            feed_pair(key.trim(), value.trim(), &current, &mut out, &mut pending);
        }
    }
    out
}

fn list_items(value: &str) -> Vec<String> {
    let inner = value.trim().trim_start_matches('[').trim_end_matches(']');
    inner
        .split(',')
        .map(|x| x.trim().trim_matches('"').to_owned())
        .filter(|x| !x.is_empty())
        .collect()
}

#[must_use]
pub fn violations(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    for section in parse_sections(content) {
        for (key, value) in &section.entries {
            let Some((want_key, want_value)) = DENY_EXCEPTION_KEYS
                .iter()
                .find(|(k, v)| *k == section.name && *v == key)
            else {
                continue;
            };
            let items = list_items(value);
            if *want_key == "advisories" && *want_value == "ignore" {
                let allowed = [ALLOWED_DENY_IGNORE.to_owned()];
                if items != allowed {
                    out.push(format!(
                        "deny.toml advisories.ignore 必須恰好是 {ALLOWED_DENY_IGNORE} 這一條，實際是 {items:?}（K75）"
                    ));
                }
            } else if !items.is_empty() {
                out.push(format!(
                    "deny.toml [{}] 的「{key}」不准有例外，實際有 {items:?}（K75）",
                    section.name
                ));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iron_laws_extra_deny_advisory_exception_is_blocked() {
        let one = "
[advisories]
yanked = 'deny'
ignore = [\"RUSTSEC-2024-0370\"]
";
        assert!(violations(one).is_empty());
        let two = "
[advisories]
ignore = [\"RUSTSEC-2024-0370\", \"RUSTSEC-2020-0001\"]
";
        assert_eq!(violations(two).len(), 1);
        let zero = "
[advisories]
ignore = []
";
        assert_eq!(violations(zero).len(), 1);
    }

    #[test]
    fn test_iron_laws_ban_and_license_exceptions_are_blocked() {
        let content = "
[bans]
multiple-versions = 'deny'
allow = [\"serde\"]
skip = []
[licenses]
allow = ['MIT', 'Apache-2.0']
exceptions = ['Unlicense']
";
        let v = violations(content);
        assert_eq!(v.len(), 2);
        assert!(v.iter().any(|x| x.contains("bans") && x.contains("allow")));
        assert!(
            v.iter()
                .any(|x| x.contains("licenses") && x.contains("exceptions"))
        );
    }

    #[test]
    fn test_iron_laws_real_repo_deny_toml_has_exactly_one_exception() {
        let repo = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let content = crate::collect::read_file_lossy(&repo.join("deny.toml"));
        assert_eq!(
            violations(&content.unwrap_or_default()),
            Vec::<String>::new()
        );
    }

    #[test]
    fn test_iron_laws_multi_line_list_is_parsed() {
        let content = "
[bans]
allow = [
  \"serde\",
]
";
        let v = violations(content);
        assert_eq!(v.len(), 1);
        assert!(v.first().is_some_and(|x| x.contains("serde")));
    }
}
