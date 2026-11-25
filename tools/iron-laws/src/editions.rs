use crate::lints::split_pair;

fn section_lines(text: &str, header: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_sec = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            in_sec = line == header;
            continue;
        }
        if in_sec && !line.is_empty() && !line.starts_with('#') {
            out.push(line.to_owned());
        }
    }
    out
}

fn edition_state(lines: &[String]) -> Option<String> {
    for line in lines {
        let Some((k, v)) = split_pair(line) else {
            continue;
        };
        if k == "edition" {
            return Some(format!("literal:{}", v.trim_matches('"')));
        }
        if k == "edition.workspace" && v == "true" {
            return Some("workspace".to_owned());
        }
    }
    None
}

#[must_use]
pub fn edition_violations(root_text: &str, manifests: &[(String, String)]) -> Vec<String> {
    let mut out = Vec::new();
    let base = edition_state(&section_lines(root_text, "[workspace.package]"));
    let Some(base) = base.filter(|b| b.starts_with("literal:")) else {
        out.push("root Cargo.toml [workspace.package] does not set edition".to_owned());
        return out;
    };
    for (rel, text) in manifests {
        match edition_state(&section_lines(text, "[package]")) {
            None => out.push(format!("{rel}: [package] does not set edition (K78)")),
            Some(s) if s == "workspace" => {}
            Some(s) if s != base => {
                let shown = s.strip_prefix("literal:").unwrap_or(&s);
                let want = base.strip_prefix("literal:").unwrap_or(&base);
                out.push(format!(
                    "{rel}: edition differs from the workspace (K78): {shown} should be {want}"
                ));
            }
            Some(_) => {}
        }
    }
    out
}
