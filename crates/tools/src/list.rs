use std::cmp::Ordering;

use anybot_protocol::{DirEntry, TargetKind};

use crate::message_codes::{CODE_TOOL_LIST_EMPTY, CODE_TOOL_LIST_MORE};

pub const MAX_LIST_ENTRIES: usize = 2_000;

const PRIVATE_EXACT_NAMES: [&str; 14] = [
    ".env",
    ".git",
    ".ssh",
    ".gnupg",
    ".aws",
    ".kube",
    ".docker",
    ".anybot",
    ".netrc",
    ".npmrc",
    ".pypirc",
    ".zsh_history",
    ".bash_history",
    "application_default_credentials.json",
];

const PRIVATE_PREFIX_NAMES: [&str; 6] = [
    ".env.",
    "id_rsa",
    "id_ed25519",
    "id_ecdsa",
    "id_dsa",
    "credentials",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingText {
    pub lines: Vec<String>,
    pub remaining: usize,
    pub is_empty: bool,
}

#[must_use]
pub fn list_more_code() -> &'static str {
    CODE_TOOL_LIST_MORE
}

#[must_use]
pub fn list_empty_code() -> &'static str {
    CODE_TOOL_LIST_EMPTY
}

#[must_use]
pub fn render_listing(entries: &[DirEntry]) -> ListingText {
    let mut visible: Vec<&DirEntry> = entries
        .iter()
        .filter(|entry| !is_private_entry(&entry.name))
        .collect();
    visible.sort_by(|a, b| compare_entries(a, b));
    let total = visible.len();
    let lines: Vec<String> = visible
        .into_iter()
        .take(MAX_LIST_ENTRIES)
        .map(format_entry)
        .collect();
    let remaining = total.saturating_sub(lines.len());
    let is_empty = lines.is_empty();
    ListingText {
        lines,
        remaining,
        is_empty,
    }
}

fn compare_entries(a: &DirEntry, b: &DirEntry) -> Ordering {
    let kind_ord = kind_rank(a.kind).cmp(&kind_rank(b.kind));
    if kind_ord != Ordering::Equal {
        return kind_ord;
    }
    let lower_ord = a.name.to_lowercase().cmp(&b.name.to_lowercase());
    if lower_ord != Ordering::Equal {
        return lower_ord;
    }
    a.name.cmp(&b.name)
}

fn kind_rank(kind: TargetKind) -> u8 {
    match kind {
        TargetKind::Directory => 0,
        TargetKind::File => 1,
    }
}

fn format_entry(entry: &DirEntry) -> String {
    match entry.kind {
        TargetKind::Directory => format!("{}/", entry.name),
        TargetKind::File => format!("{} ({})", entry.name, format_size(entry.size)),
    }
}

#[must_use]
pub fn format_size(bytes: u64) -> String {
    if bytes < 1_024 {
        return format!("{bytes} B");
    }
    if bytes < 1_048_576 {
        let kb = bytes.saturating_add(512) / 1_024;
        return format!("{kb} KB");
    }
    let mb = bytes.saturating_add(524_288) / 1_048_576;
    format!("{mb} MB")
}

#[must_use]
pub fn is_private_entry(name: &str) -> bool {
    let trimmed = name.trim_end_matches('/');
    if anybot_trust::is_private_path(trimmed, "", "") {
        return true;
    }
    let lower = trimmed.to_lowercase();
    let has_secret_ext = std::path::Path::new(trimmed)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("pem") || ext.eq_ignore_ascii_case("key"));
    PRIVATE_EXACT_NAMES.contains(&lower.as_str())
        || PRIVATE_PREFIX_NAMES
            .iter()
            .any(|prefix| lower.starts_with(prefix))
        || has_secret_ext
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str) -> DirEntry {
        DirEntry {
            name: name.to_owned(),
            kind: TargetKind::Directory,
            size: 0,
        }
    }

    fn file(name: &str, size: u64) -> DirEntry {
        DirEntry {
            name: name.to_owned(),
            kind: TargetKind::File,
            size,
        }
    }

    #[test]
    fn listing_sorts_dirs_first_case_insensitive() {
        let out = render_listing(&[file("C.txt", 0), dir("b"), file("a.txt", 3), dir("a")]);
        assert_eq!(
            out.lines,
            vec![
                "a/".to_owned(),
                "b/".to_owned(),
                "a.txt (3 B)".to_owned(),
                "C.txt (0 B)".to_owned(),
            ]
        );
        assert_eq!(out.remaining, 0);
        assert!(!out.is_empty);
    }

    #[test]
    fn listing_hides_private_names() {
        let out = render_listing(&[
            file(".env", 10),
            file(".env.local", 10),
            file("id_rsa", 10),
            file("cert.pem", 10),
            file("priv.key", 10),
            dir(".git"),
            file("notes.txt", 3),
        ]);
        assert_eq!(out.lines, vec!["notes.txt (3 B)".to_owned()]);
        assert_eq!(out.remaining, 0);
    }

    #[test]
    fn listing_truncates_at_limit_and_marks_empty() {
        let empty = render_listing(&[]);
        assert!(empty.is_empty);
        assert_eq!(empty.remaining, 0);
        assert!(empty.lines.is_empty());

        let many: Vec<DirEntry> = (0..2_005)
            .map(|n| file(&format!("f{n:04}.txt"), 0))
            .collect();
        let out = render_listing(&many);
        assert_eq!(out.lines.len(), MAX_LIST_ENTRIES);
        assert_eq!(out.remaining, 5);
        assert!(!out.is_empty);
    }
}
