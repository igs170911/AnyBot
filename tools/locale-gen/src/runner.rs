use std::path::{Path, PathBuf};
use std::process::Command;

pub const RUST_CODES_REL: &str = "crates/messages/src/generated/codes.rs";
pub const TS_CODES_REL: &str = "web/ui/src/generated/codes.ts";
pub const TS_LOCALES_REL: &str = "web/ui/src/generated/locales.ts";

#[derive(Debug)]
pub enum RunnerError {
    Git(String),
    Io(String),
}

impl std::fmt::Display for RunnerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Git(reason) => write!(f, "git failed: {reason}"),
            Self::Io(reason) => write!(f, "io failed: {reason}"),
        }
    }
}

impl std::error::Error for RunnerError {}

pub fn repo_root() -> Result<PathBuf, RunnerError> {
    let out = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| RunnerError::Git(e.to_string()))?;
    if !out.status.success() {
        return Err(RunnerError::Git(
            String::from_utf8_lossy(&out.stderr).into_owned(),
        ));
    }
    let text = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    Ok(PathBuf::from(text))
}

pub fn tracked_files(root: &Path) -> Result<Vec<String>, RunnerError> {
    const FLAGS: [[&str; 5]; 2] = [
        ["ls-files", "-z", "-c", "*", "*"],
        ["ls-files", "-z", "-o", "--exclude-standard", "--directory"],
    ];
    let mut out = Vec::new();
    for flags in FLAGS {
        let got = Command::new("git")
            .args(flags)
            .current_dir(root)
            .output()
            .map_err(|e| RunnerError::Git(e.to_string()))?;
        if !got.status.success() {
            return Err(RunnerError::Git(
                String::from_utf8_lossy(&got.stderr).into_owned(),
            ));
        }
        let text = String::from_utf8_lossy(&got.stdout).into_owned();
        out.extend(
            text.split('\0')
                .filter(|p| !p.is_empty() && !p.ends_with('/'))
                .map(ToOwned::to_owned),
        );
    }
    Ok(out)
}

pub fn read_rel(root: &Path, rel: &str) -> Result<String, RunnerError> {
    std::fs::read_to_string(root.join(rel)).map_err(|e| RunnerError::Io(format!("{rel}: {e}")))
}

pub fn write_rel(root: &Path, rel: &str, content: &str) -> Result<(), RunnerError> {
    if let Some(dir) = Path::new(rel).parent() {
        std::fs::create_dir_all(root.join(dir)).map_err(|e| RunnerError::Io(e.to_string()))?;
    }
    std::fs::write(root.join(rel), content).map_err(|e| RunnerError::Io(format!("{rel}: {e}")))
}

pub fn read_all(root: &Path, paths: &[String]) -> Result<Vec<(String, String)>, RunnerError> {
    let mut out = Vec::new();
    for path in paths {
        if is_binary_path(path) || Path::new(path).is_symlink() {
            continue;
        }
        out.push((path.clone(), read_rel(root, path)?));
    }
    Ok(out)
}

#[must_use]
pub fn is_binary_path(path: &str) -> bool {
    const BINARY: [&str; 7] = ["png", "ico", "icns", "jpg", "jpeg", "gif", "pdf"];
    Path::new(path)
        .extension()
        .is_some_and(|ext| BINARY.iter().any(|b| ext.eq_ignore_ascii_case(b)))
}

#[must_use]
pub fn is_source_path(path: &str) -> bool {
    (has_extension(path, "rs") || has_extension(path, "ts") || has_extension(path, "tsx"))
        && !is_scan_skipped_path(path)
}

fn has_extension(path: &str, want: &str) -> bool {
    Path::new(path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case(want))
}

fn is_scan_skipped_path(path: &str) -> bool {
    const SKIPPED: [&str; 5] = [
        "node_modules/",
        "/dist/",
        "/generated/",
        "/target/",
        "locales/",
    ];
    SKIPPED.iter().any(|p| path.contains(p))
}

#[must_use]
pub fn split_scan(entries: &[(String, String)]) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut rust = Vec::new();
    let mut ts = Vec::new();
    let mut all = Vec::new();
    for (path, content) in entries {
        all.push(content.clone());
        if has_extension(path, "rs") {
            rust.push(content.clone());
        }
        if has_extension(path, "ts") || has_extension(path, "tsx") {
            ts.push(content.clone());
        }
    }
    (rust, ts, all)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_path_filters_extensions_and_skipped_dirs() {
        assert!(is_source_path("crates/messages/src/catalog.rs"));
        assert!(is_source_path("web/ui/src/shell.tsx"));
        assert!(!is_source_path("Makefile"));
        assert!(!is_source_path("locales/en.ftl"));
        assert!(!is_source_path("web/ui/src/generated/codes.ts"));
        assert!(!is_source_path("node_modules/pkg/index.ts"));
        assert!(!is_source_path("web/ui/dist/main.ts"));
    }

    #[test]
    fn test_binary_path_skips_image_icon_and_pdf_extensions() {
        assert!(is_binary_path("apps/desktop/src-tauri/icons/icon.icns"));
        assert!(is_binary_path("apps/desktop/src-tauri/icons/icon.png"));
        assert!(is_binary_path("docs/manual.pdf"));
        assert!(!is_binary_path("crates/messages/src/lang.rs"));
        assert!(!is_binary_path("locales/en.ftl"));
    }

    #[test]
    fn test_split_scan_routes_rust_and_ts_contents() {
        let entries = vec![
            ("a/x.rs".to_owned(), "MessageCode::ACode".to_owned()),
            ("a/y.ts".to_owned(), "\"b-code\"".to_owned()),
            ("a/z.md".to_owned(), "text".to_owned()),
        ];
        let (rust, ts, all) = split_scan(&entries);
        assert_eq!(rust, vec!["MessageCode::ACode".to_owned()]);
        assert_eq!(ts, vec!["\"b-code\"".to_owned()]);
        assert_eq!(all.len(), 3);
    }
}
