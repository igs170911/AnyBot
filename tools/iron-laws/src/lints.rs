use std::path::Path;

use crate::collect::read_file_lossy;
use crate::collect::{dir_name, is_skipped_dir, path_to_rel};

pub const CONTEXT_PKG: &str = "anybot-app-desktop-context";
pub const CONTEXT_SRC_DIR: &str = "apps/desktop/context/src";
fn norm(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}
pub(crate) fn split_pair(line: &str) -> Option<(String, String)> {
    let (k, v) = line.split_once('=')?;
    let k = k.trim();
    let v = v.trim();
    if k.is_empty() || v.is_empty() {
        return None;
    }
    Some((k.to_owned(), norm(v)))
}
#[must_use]
pub fn lint_entries(text: &str, rust_h: &str, clippy_h: &str) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    let mut section: Option<&str> = None;
    for raw in text.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            section = if line == rust_h {
                Some("rust")
            } else if line == clippy_h {
                Some("clippy")
            } else {
                None
            };
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = split_pair(line) else {
            continue;
        };
        if let Some(sec) = section {
            out.push((sec.to_owned(), k, v));
        }
    }
    out
}
fn workspace_lints(root: &str) -> Vec<(String, String, String)> {
    lint_entries(root, "[workspace.lints.rust]", "[workspace.lints.clippy]")
}
fn package_lints(text: &str) -> Vec<(String, String, String)> {
    lint_entries(text, "[lints.rust]", "[lints.clippy]")
}
#[must_use]
pub fn inherits_workspace(text: &str) -> bool {
    let mut in_lints = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            in_lints = line == "[lints]";
            continue;
        }
        if in_lints && line == "workspace = true" {
            return true;
        }
    }
    false
}
fn minus_exit(entries: &[(String, String, String)]) -> Vec<(String, String, String)> {
    entries
        .iter()
        .filter(|e| !(e.0 == "clippy" && e.1 == "exit"))
        .cloned()
        .collect()
}
fn package_name(text: &str) -> Option<String> {
    let mut in_package = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            in_package = line == "[package]";
            continue;
        }
        if in_package && line.starts_with("name =") {
            let (_, v) = split_pair(line)?;
            return Some(v.trim_matches('"').to_owned());
        }
    }
    None
}
fn lint_diff(
    rel: &str,
    pkg: &[(String, String, String)],
    expected: &[(String, String, String)],
) -> Vec<String> {
    let mut out = Vec::new();
    for e in pkg {
        if !expected.contains(e) {
            out.push(format!(
                "{rel}：lint 跟 workspace 不一致，多了 {}.{} = {}",
                e.0, e.1, e.2
            ));
        }
    }
    for e in expected {
        if !pkg.contains(e) {
            out.push(format!(
                "{rel}：lint 跟 workspace 不一致，少了 {}.{} = {}",
                e.0, e.1, e.2
            ));
        }
    }
    out
}
fn is_fn_def_line(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("fn ") || t.starts_with("pub fn ") || t.starts_with("pub(crate) fn ")
}
#[must_use]
pub fn ctx_function_count(files: &[(String, String)]) -> usize {
    let mut n: usize = 0;
    for (path, content) in files {
        if !path.starts_with(CONTEXT_SRC_DIR) {
            continue;
        }
        for line in content.lines() {
            if is_fn_def_line(line) {
                n = n.saturating_add(1);
            }
        }
    }
    n
}
fn check_context(
    rel: &str,
    text: &str,
    ws: &[(String, String, String)],
    files: &[(String, String)],
) -> Vec<String> {
    let mut out = Vec::new();
    if inherits_workspace(text) {
        out.push(format!(
            "{rel}：小套件不准用 [lints] workspace = true（Cargo 不支援覆蓋，要完整列出來）"
        ));
    }
    out.extend(lint_diff(rel, &package_lints(text), &minus_exit(ws)));
    let n = ctx_function_count(files);
    if n != 1 {
        out.push(format!(
            "{CONTEXT_PKG} 的程式只准有那一個函式，現在有 {n} 個（spec/09 K82）"
        ));
    }
    out
}
#[must_use]
pub fn check_member(
    rel: &str,
    text: &str,
    ws: &[(String, String, String)],
    files: &[(String, String)],
) -> Vec<String> {
    if package_name(text).as_deref() == Some(CONTEXT_PKG) {
        return check_context(rel, text, ws, files);
    }
    if inherits_workspace(text) {
        return Vec::new();
    }
    vec![format!(
        "{rel}：沒有 [lints] workspace = true，必須完整繼承 workspace 的 lint（spec/09 K82）"
    )]
}
#[must_use]
pub fn check_members(
    manifests: &[(String, String)],
    ws: &[(String, String, String)],
    files: &[(String, String)],
) -> Vec<String> {
    let mut out = Vec::new();
    for (rel, text) in manifests {
        out.extend(check_member(rel, text, ws, files));
    }
    out
}
fn clean_entry(line: &str) -> Option<String> {
    let t = line.trim().trim_end_matches(',').trim_matches('"');
    if t.is_empty() {
        return None;
    }
    Some(t.to_owned())
}
#[must_use]
pub fn member_globs(root_text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_members = false;
    for raw in root_text.lines() {
        let line = raw.trim();
        if line.starts_with("members =") {
            in_members = true;
            continue;
        }
        if in_members && line == "]" {
            break;
        }
        if in_members && let Some(g) = clean_entry(line) {
            out.push(g);
        }
    }
    out
}
fn expand_globs(globs: &[String], repo: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for g in globs {
        if let Some(parent) = g.strip_suffix("/*") {
            let Ok(entries) = std::fs::read_dir(repo.join(parent)) else {
                continue;
            };
            for entry in entries.flatten() {
                let Some(name) = dir_name(&entry.path()) else {
                    continue;
                };
                let rel = format!("{parent}/{name}/Cargo.toml");
                if repo.join(&rel).is_file() {
                    out.push(rel);
                }
            }
            continue;
        }
        let rel = format!("{g}/Cargo.toml");
        if repo.join(&rel).is_file() {
            out.push(rel);
        }
    }
    out.sort();
    out
}
fn find_allowlists(dir: &Path, repo: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = dir_name(&path) else {
            continue;
        };
        if name == ".gitleaks.toml" {
            if let Some(rel) = path_to_rel(&path, repo) {
                out.push(rel);
            }
            continue;
        }
        if path.is_dir() && !is_skipped_dir(&name) {
            find_allowlists(&path, repo, out);
        }
    }
}
#[must_use]
pub fn gitleaks_violations(repo: &Path) -> Vec<String> {
    let mut rels = Vec::new();
    find_allowlists(repo, repo, &mut rels);
    rels.sort();
    rels.iter()
        .map(|r| format!("{r}：掃金鑰工具不准有任何放行清單（spec/09 K83）"))
        .collect()
}
#[must_use]
pub fn violations(repo: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(root) = read_file_lossy(&repo.join("Cargo.toml")) else {
        return vec!["讀根目錄 Cargo.toml 失敗".to_owned()];
    };
    let ws = workspace_lints(&root);
    if ws.is_empty() {
        out.push("根目錄 Cargo.toml 沒有 workspace lints".to_owned());
    }
    let manifests = expand_globs(&member_globs(&root), repo);
    let mut manifest_texts = Vec::new();
    for rel in manifests {
        if let Ok(text) = read_file_lossy(&repo.join(&rel)) {
            manifest_texts.push((rel, text));
        }
    }
    out.extend(check_members(
        &manifest_texts,
        &ws,
        &crate::collect::source_files(repo),
    ));
    out.extend(crate::editions::edition_violations(&root, &manifest_texts));
    out.extend(gitleaks_violations(repo));
    out
}
#[cfg(test)]
#[path = "lints_tests.rs"]
mod tests;
