use std::path::Path;

use crate::collect::dir_name;
use crate::collect::read_file_lossy;
use crate::collect::{is_skipped_dir, path_to_rel};

fn is_redocly_config(name: &str) -> bool {
    let ext_ok = name
        .rsplit_once('.')
        .is_some_and(|(_, ext)| ext.eq_ignore_ascii_case("yaml"));
    name == "redocly.yaml" || (name.starts_with("redocly-") && ext_ok)
}

fn walk_matching(dir: &Path, repo: &Path, want: &dyn Fn(&str) -> bool, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = dir_name(&path) else {
            continue;
        };
        if want(&name) {
            if let Some(rel) = path_to_rel(&path, repo) {
                out.push(rel);
            }
            continue;
        }
        if path.is_dir() && !is_skipped_dir(&name) {
            walk_matching(&path, repo, want, out);
        }
    }
}

fn lint_ignore_rels(repo: &Path) -> Vec<String> {
    let mut rels = Vec::new();
    walk_matching(repo, repo, &|n| n == ".redocly.lint-ignore.yaml", &mut rels);
    rels.sort();
    rels
}

fn redocly_config_rels(repo: &Path) -> Vec<String> {
    let mut rels = Vec::new();
    walk_matching(
        repo,
        repo,
        &is_redocly_config as &dyn Fn(&str) -> bool,
        &mut rels,
    );
    rels.sort();
    rels
}

fn soft_severity_lines(text: &str) -> Vec<String> {
    let mut hits = Vec::new();
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('#') {
            continue;
        }
        let Some((_, v)) = t.split_once(':') else {
            continue;
        };
        let v = v.trim().trim_matches('"').trim_matches('\'');
        if v == "off" || v == "warn" {
            hits.push(t.to_owned());
        }
    }
    hits
}

#[must_use]
pub fn violations(repo: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for rel in lint_ignore_rels(repo) {
        out.push(format!(
            "{rel}: redocly ignore files must not exist; every rule must be error (second review item 5)"
        ));
    }
    for rel in redocly_config_rels(repo) {
        let Ok(text) = read_file_lossy(&repo.join(&rel)) else {
            continue;
        };
        for line in soft_severity_lines(&text) {
            out.push(format!(
                "{rel}: redocly config has a non-error rule ({line}); set it to error (second review item 5)"
            ));
        }
    }
    out
}

#[cfg(test)]
#[path = "redocly_tests.rs"]
mod tests;
