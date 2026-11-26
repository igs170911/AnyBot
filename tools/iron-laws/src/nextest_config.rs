use std::path::Path;

use crate::collect::read_file_lossy;

pub const NEXTEST_CONFIG_REL: &str = ".config/nextest.toml";

#[must_use]
pub fn violations(repo: &Path) -> Vec<String> {
    let path = repo.join(NEXTEST_CONFIG_REL);
    if !path.is_file() {
        return vec![format!(
            "{NEXTEST_CONFIG_REL}: missing; nextest runaway tests must be terminated automatically (AGENTS section 3)"
        )];
    }
    let Ok(content) = read_file_lossy(&path) else {
        return vec![format!("{NEXTEST_CONFIG_REL}: cannot be read")];
    };
    let default_body = section_body(&content, "[profile.default]");
    let Some(body) = default_body else {
        return vec![format!(
            "{NEXTEST_CONFIG_REL}: has no [profile.default] section (AGENTS section 3)"
        )];
    };
    if !body.contains("terminate-after") {
        return vec![format!(
            "{NEXTEST_CONFIG_REL}: [profile.default] lacks terminate-after; slow-timeout must terminate runaway tests (AGENTS section 3)"
        )];
    }
    let mut out = unknown_shape_violations(&content);
    out.extend(quick_postgres_violations(repo, &content));
    out
}

fn is_rust_source(name: &str) -> bool {
    Path::new(name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("rs"))
}

#[must_use]
pub fn quick_filter_of(content: &str) -> Option<String> {
    quick_filter(content)
}

fn quick_filter(content: &str) -> Option<String> {
    let body = section_body(content, "[profile.quick]")?;
    for line in body.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() == "default-filter" {
            return Some(value.trim().trim_matches('\'').trim_matches('"').to_owned());
        }
    }
    None
}

fn quick_postgres_violations(repo: &Path, content: &str) -> Vec<String> {
    let mut out = Vec::new();
    let Some(filter) = quick_filter(content) else {
        return vec![format!(
            "{NEXTEST_CONFIG_REL}: [profile.quick] has no default-filter; basic tests must skip pg_ binaries"
        )];
    };
    if !filter.contains("binary(/^pg_/)") {
        out.push(format!(
            "{NEXTEST_CONFIG_REL}: quick profile must exclude binary(/^pg_/) (docs/testing.md)"
        ));
    }
    for root_name in ["tests", "apps", "crates"] {
        scan_pg_start(&repo.join(root_name), repo, &mut out);
    }
    out
}

fn scan_pg_start(dir: &Path, repo: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if name == "target" || name == "node_modules" {
                continue;
            }
            scan_pg_start(&path, repo, out);
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !is_rust_source(name) || name.starts_with("pg_") {
            continue;
        }
        let Some(parent) = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
        else {
            continue;
        };
        if parent != "tests" {
            continue;
        }
        let Ok(text) = read_file_lossy(&path) else {
            continue;
        };
        if !text.contains("pg_start(") {
            continue;
        }
        let rel = match path.strip_prefix(repo) {
            Ok(p) => p.display().to_string(),
            Err(_) => name.to_owned(),
        };
        out.push(format!(
            "{rel}: calls pg_start but is not named pg_*.rs (docs/testing.md)"
        ));
    }
}

fn section_body(content: &str, header: &str) -> Option<String> {
    let mut body = String::new();
    let mut inside = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            if trimmed == header {
                inside = true;
            } else if inside {
                break;
            }
            continue;
        }
        if inside {
            body.push_str(trimmed);
            body.push('\n');
        }
    }
    if body.is_empty() {
        return None;
    }
    Some(body)
}

fn is_header(line: &str, header: &str) -> bool {
    line.trim() == header
}

fn collect_section(content: &str, header: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut inside = false;
    for line in content.lines() {
        if is_header(line, header) {
            inside = true;
            continue;
        }
        if line.trim().starts_with('[') {
            inside = false;
            continue;
        }
        if inside && !line.trim().is_empty() {
            lines.push(line.trim().to_owned());
        }
    }
    lines
}

fn unknown_shape_violations(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    if content.contains("[[profile.default.serial]]") {
        out.push(format!(
            "{NEXTEST_CONFIG_REL}: [[profile.default.serial]] is not a nextest key; serial work must use [test-groups] (AGENTS section 3)"
        ));
    }
    if !collect_section(content, "[profile.default.test-groups]").is_empty() {
        out.push(format!(
            "{NEXTEST_CONFIG_REL}: [profile.default.test-groups] is not a nextest key; groups belong in top-level [test-groups] (AGENTS section 3)"
        ));
    }
    for line in collect_section(content, "[[profile.default.overrides]]") {
        let Some((key, _rest)) = line.split_once('=') else {
            continue;
        };
        if key.trim() == "group" {
            out.push(format!(
                "{NEXTEST_CONFIG_REL}: overrides use \"group\" which is not a nextest key; assignment is test-group (AGENTS section 3)"
            ));
        }
    }
    out
}
