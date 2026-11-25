use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::collect::{is_skipped_dir, is_source_file, read_file_lossy};
use crate::table::PACKAGE_SCAN_DIRS;

pub fn cargo_metadata(repo: &Path) -> Result<String, String> {
    let output = std::process::Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .current_dir(repo)
        .output()
        .map_err(|e| format!("failed to run cargo metadata: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    String::from_utf8(output.stdout).map_err(|e| format!("cargo metadata is not utf8: {e}"))
}

fn package_roots(metadata_json: &str) -> Result<Vec<PathBuf>, String> {
    let value: serde_json::Value =
        serde_json::from_str(metadata_json).map_err(|e| format!("cargo metadata json: {e}"))?;
    let packages = value
        .get("packages")
        .and_then(serde_json::Value::as_array)
        .ok_or("cargo metadata has no packages array".to_owned())?;
    let mut roots = Vec::new();
    for package in packages {
        let Some(manifest) = package
            .get("manifest_path")
            .and_then(serde_json::Value::as_str)
        else {
            return Err("cargo metadata package lacks manifest_path".to_owned());
        };
        let manifest = PathBuf::from(manifest);
        let Some(root) = manifest.parent() else {
            return Err(format!("manifest {} has no parent dir", manifest.display()));
        };
        roots.push(root.to_path_buf());
    }
    Ok(roots)
}

fn collect_package_rs_files(root_dir: &Path, repo: &Path, out: &mut Vec<String>) {
    for scan in PACKAGE_SCAN_DIRS {
        let dir = root_dir.join(scan);
        if dir.is_dir() {
            walk_rs(&dir, repo, out);
        }
    }
}

fn walk_rs(dir: &Path, repo: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_owned();
        if path.is_dir() {
            if !is_skipped_dir(&name) {
                walk_rs(&path, repo, out);
            }
            continue;
        }
        if !is_source_file(&path) {
            continue;
        }
        let rel = path.strip_prefix(repo).unwrap_or(&path);
        out.push(rel.to_string_lossy().into_owned());
    }
}

#[must_use]
pub fn compiled_tokens(repo: &Path) -> BTreeSet<String> {
    let mut tokens = BTreeSet::new();
    let repo_text = repo.to_string_lossy().into_owned();
    let deps = repo.join("target").join("debug").join("deps");
    let Ok(entries) = std::fs::read_dir(&deps) else {
        return tokens;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path
            .extension()
            .is_none_or(|e| !e.eq_ignore_ascii_case("d"))
        {
            continue;
        }
        let Ok(content) = read_file_lossy(&path) else {
            continue;
        };
        for line in content.lines() {
            for token in line.split_whitespace() {
                let cleaned = token.trim_end_matches(':').to_owned();
                if cleaned
                    .rsplit_once('.')
                    .is_some_and(|(_stem, ext)| ext.eq_ignore_ascii_case("rs"))
                {
                    let rel = strip_repo(&cleaned, &repo_text);
                    tokens.insert(rel);
                }
            }
        }
    }
    tokens
}

fn strip_repo(token: &str, repo_text: &str) -> String {
    let trimmed = token.trim_start_matches(repo_text);
    trimmed.trim_start_matches('/').to_owned()
}

#[must_use]
pub fn violations(repo: &Path, metadata_json: &str, compiled: &BTreeSet<String>) -> Vec<String> {
    if compiled.is_empty() {
        return vec![
            "compile gate: no dep-info files under target/debug/deps; run cargo check --all-targets first"
                .to_owned(),
        ];
    }
    let roots = match package_roots(metadata_json) {
        Ok(roots) => roots,
        Err(e) => return vec![e],
    };
    let mut out = Vec::new();
    for root_dir in roots {
        let mut files = Vec::new();
        collect_package_rs_files(&root_dir, repo, &mut files);
        for rel in files {
            if compiled.contains(&rel) {
                continue;
            }
            out.push(format!(
                "{rel}: source file is not compiled by any target (M4 A.1: add it to the module tree, tests, benches, or examples)"
            ));
        }
    }
    out.sort();
    out
}

#[must_use]
pub fn violations_at_repo(repo: &Path) -> Vec<String> {
    let repo = &repo.canonicalize().unwrap_or_else(|_e| repo.to_path_buf());
    let metadata = match cargo_metadata(repo) {
        Ok(text) => text,
        Err(e) => return vec![e],
    };
    violations(repo, &metadata, &compiled_tokens(repo))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> (PathBuf, String, BTreeSet<String>) {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/scratch");
        let repo = root.join(name);
        let pkg = repo.join("crates").join("widget");
        std::fs::create_dir_all(pkg.join("src")).unwrap();
        std::fs::create_dir_all(pkg.join("tests")).unwrap();
        std::fs::create_dir_all(pkg.join("benches")).unwrap();
        std::fs::write(pkg.join("src").join("lib.rs"), "fn main() {}\n").unwrap();
        std::fs::write(pkg.join("src").join("orphan.rs"), "fn lonely() {}\n").unwrap();
        std::fs::write(pkg.join("tests").join("wired.rs"), "fn t() {}\n").unwrap();
        std::fs::write(pkg.join("benches").join("slow.rs"), "fn b() {}\n").unwrap();
        let manifest = pkg.join("Cargo.toml");
        std::fs::write(
            &manifest,
            "[package]\nname = \"widget\"\nversion = \"0.0.1\"\nedition = \"2024\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(repo.join("target").join("debug").join("deps")).unwrap();
        std::fs::write(
            repo.join("target")
                .join("debug")
                .join("deps")
                .join("widget-1.d"),
            "out.d: crates/widget/src/lib.rs crates/widget/tests/wired.rs\n",
        )
        .unwrap();
        let metadata = format!(
            "{{\"packages\":[{{\"name\":\"widget\",\"manifest_path\":\"{}\"}}]}}",
            manifest.display()
        );
        (repo.clone(), metadata, compiled_tokens(&repo))
    }

    #[test]
    fn test_compiled_gate_blocks_unwired_source_file() {
        let (repo, metadata, compiled) = fixture("iron-compiled-block");
        let v = violations(&repo, &metadata, &compiled);
        assert!(
            v.iter()
                .any(|x| x.contains("crates/widget/src/orphan.rs") && x.contains("not compiled")),
            "{v:?}"
        );
        assert!(
            v.iter()
                .any(|x| x.contains("crates/widget/benches/slow.rs")),
            "{v:?}"
        );
        assert!(
            !v.iter().any(|x| x.contains("src/lib.rs")),
            "compiled file must not be flagged: {v:?}"
        );
        assert!(
            !v.iter().any(|x| x.contains("tests/wired.rs")),
            "compiled test file must not be flagged: {v:?}"
        );
    }

    #[test]
    fn test_compiled_gate_needs_dep_info_files() {
        let (repo, metadata, _compiled) = fixture("iron-compiled-empty");
        std::fs::remove_dir_all(repo.join("target").join("debug").join("deps")).unwrap();
        let compiled = compiled_tokens(&repo);
        let v = violations(&repo, &metadata, &compiled);
        assert_eq!(v.len(), 1);
        assert!(
            v.first().is_some_and(|x| x.contains("no dep-info files")),
            "{v:?}"
        );
    }

    #[test]
    fn test_compiled_gate_covers_the_real_repo() {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let v = violations_at_repo(&repo);
        assert_eq!(v, Vec::<String>::new(), "{v:?}");
    }
}
