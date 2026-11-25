use std::path::Path;

use crate::collect::dir_name;
use crate::collect::read_file_lossy;
use crate::collect::{is_skipped_dir, path_to_rel};

pub const DUAL_LICENSE: &str = "MIT OR Apache-2.0";

fn walk_named(dir: &Path, repo: &Path, name: &str, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(found) = dir_name(&path) else {
            continue;
        };
        if found == name {
            if let Some(rel) = path_to_rel(&path, repo) {
                out.push(rel);
            }
            continue;
        }
        if path.is_dir() && !is_skipped_dir(&found) {
            walk_named(&path, repo, name, out);
        }
    }
}

fn cargo_tomls(repo: &Path) -> Vec<String> {
    let mut rels = vec!["Cargo.toml".to_owned()];
    walk_named(repo, repo, "Cargo.toml", &mut rels);
    rels.sort();
    rels.dedup();
    rels
}

fn package_jsons(repo: &Path) -> Vec<String> {
    let mut rels = Vec::new();
    walk_named(repo, repo, "package.json", &mut rels);
    rels.sort();
    rels
}

fn tauri_confs(repo: &Path) -> Vec<String> {
    let mut rels = Vec::new();
    walk_named(repo, repo, "tauri.conf.json", &mut rels);
    rels.sort();
    rels
}

fn cargo_license_violations(repo: &Path, rels: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for rel in rels {
        let Ok(text) = read_file_lossy(&repo.join(rel)) else {
            continue;
        };
        if rel == "Cargo.toml" {
            if !text.contains(&format!("license = \"{DUAL_LICENSE}\"")) {
                out.push(format!(
                    "{rel}：workspace 授權欄位必須是 {DUAL_LICENSE}（spec/09 K88）"
                ));
            }
            continue;
        }
        if !text.contains("license.workspace = true") {
            out.push(format!("{rel}：授權欄位必須繼承 workspace（spec/09 K88）"));
        }
    }
    out
}

fn json_license_violations(repo: &Path, rels: &[String]) -> Vec<String> {
    let want: String = format!("\"license\":\"{DUAL_LICENSE}\"")
        .split_whitespace()
        .collect();
    let mut out = Vec::new();
    for rel in rels {
        let Ok(text) = read_file_lossy(&repo.join(rel)) else {
            continue;
        };
        let flat: String = text.split_whitespace().collect();
        if !flat.contains(&want) {
            out.push(format!(
                "{rel}：授權欄位必須是 {DUAL_LICENSE}（spec/09 K88）"
            ));
        }
    }
    out
}

#[must_use]
pub fn violations(repo: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for name in ["LICENSE-MIT", "LICENSE-APACHE"] {
        if !repo.join(name).exists() {
            out.push(format!(
                "{name}：開源授權全文檔必須在 repo 根目錄（spec/09 K88）"
            ));
        }
    }
    out.extend(cargo_license_violations(repo, &cargo_tomls(repo)));
    out.extend(json_license_violations(repo, &package_jsons(repo)));
    out.extend(json_license_violations(repo, &tauri_confs(repo)));
    out
}

#[cfg(test)]
#[path = "licensing_tests.rs"]
mod tests;
