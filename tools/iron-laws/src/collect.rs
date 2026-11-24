use std::io::Read;
use std::path::Path;

use crate::table::{SCAN_DIRS, SKIP_DIR_NAMES, SOURCE_EXTENSIONS};

#[must_use]
pub fn is_source_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| SOURCE_EXTENSIONS.iter().any(|x| e == *x))
}

#[must_use]
pub fn is_skipped_dir(name: &str) -> bool {
    SKIP_DIR_NAMES.contains(&name)
}

pub fn read_file_lossy(path: &Path) -> Result<String, String> {
    let mut file =
        std::fs::File::open(path).map_err(|e| format!("讀 {} 失敗：{e}", path.display()))?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)
        .map_err(|e| format!("讀 {} 失敗：{e}", path.display()))?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

fn dir_name(path: &Path) -> Option<String> {
    Some(path.file_name()?.to_str()?.to_owned())
}

fn walk(dir: &Path, repo: &Path, out: &mut Vec<(String, String)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = dir_name(&path) else {
            continue;
        };
        if path.is_dir() {
            if !is_skipped_dir(&name) {
                walk(&path, repo, out);
            }
            continue;
        }
        if !is_source_file(&path) {
            continue;
        }
        let Ok(content) = read_file_lossy(&path) else {
            continue;
        };
        let Some(rel) = path_to_rel(&path, repo) else {
            continue;
        };
        out.push((rel, content));
    }
}

#[must_use]
pub fn source_files(repo: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for dir in SCAN_DIRS {
        walk(&repo.join(dir), repo, &mut out);
    }
    out.sort();
    out
}

fn path_to_rel(path: &Path, repo: &Path) -> Option<String> {
    Some(
        path.to_str()?
            .strip_prefix(&format!("{}/", repo.to_str()?))?
            .to_owned(),
    )
}

fn find_clippy_tomls(dir: &Path, repo: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = dir_name(&path) else {
            continue;
        };
        if is_skipped_dir(&name) {
            continue;
        }
        let candidate = path.join("clippy.toml");
        if candidate.is_file()
            && let Some(rel) = path_to_rel(&candidate, repo)
        {
            out.push(rel);
        }
        find_clippy_tomls(&path, repo, out);
    }
}

#[must_use]
pub fn clippy_toml_files(repo: &Path) -> Vec<(String, String)> {
    let mut rels = Vec::new();
    for dir in ["crates", "apps", "tools"] {
        find_clippy_tomls(&repo.join(dir), repo, &mut rels);
    }
    rels.sort();
    let mut out = Vec::new();
    for rel in rels {
        if let Ok(content) = read_file_lossy(&repo.join(&rel)) {
            out.push((rel, content));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_collect_source_file_extensions_and_skips() {
        assert!(is_source_file(Path::new("a/b.rs")));
        assert!(is_source_file(Path::new("a/b.tsx")));
        assert!(!is_source_file(Path::new("a/b.js")));
        assert!(!is_source_file(Path::new("a/b.md")));
        assert!(is_skipped_dir("node_modules"));
        assert!(is_skipped_dir("generated"));
        assert!(!is_skipped_dir("src"));
    }

    #[test]
    fn test_collect_path_to_rel_strips_repo_root() {
        let repo = Path::new("/root");
        assert_eq!(
            path_to_rel(Path::new("/root/crates/x/clippy.toml"), repo).as_deref(),
            Some("crates/x/clippy.toml")
        );
        assert_eq!(path_to_rel(Path::new("/other/y"), repo), None);
    }

    #[test]
    fn test_collect_source_files_skips_generated_dirs() {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/scratch/iron");
        assert!(std::fs::create_dir_all(repo.join("crates/x/src/generated/deep")).is_ok());
        assert!(std::fs::create_dir_all(repo.join("crates/x/src/real")).is_ok());
        assert!(
            std::fs::write(
                repo.join("crates/x/src/generated/deep/a.rs"),
                "fn main() {}\n",
            )
            .is_ok()
        );
        assert!(std::fs::write(repo.join("crates/x/src/real/b.rs"), "fn main() {}\n").is_ok());
        let files = source_files(&repo);
        let names: Vec<String> = files.iter().map(|(p, _)| p.clone()).collect();
        assert!(names.iter().any(|p| p.ends_with("real/b.rs")));
        assert!(!names.iter().any(|p| p.contains("generated")));
    }
}
