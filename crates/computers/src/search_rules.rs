use std::path::Path;

use anybot_protocol::SearchEngine;
use anybot_trust::is_private_path;

pub const SEARCH_MAX_MS: u64 = 25_000;
pub const SEARCH_MAX_LINES: usize = 10_000;
pub const SEARCH_RETURN_LINES: usize = 2_000;
pub const SEARCH_LINE_CHARS: usize = 300;

const PRIVATE_EXACT_SEGMENTS: [&str; 14] = [
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

const PRIVATE_PREFIX_SEGMENTS: [&str; 6] = [
    ".env.",
    "id_rsa",
    "id_ed25519",
    "id_ecdsa",
    "id_dsa",
    "credentials",
];

const PLAIN_ONLY_SKIP_SEGMENTS: [&str; 2] = ["node_modules", "target"];

#[must_use]
pub fn skipped(engine: SearchEngine, path: &str) -> bool {
    if is_private_path(path, "", "") {
        return true;
    }
    for segment in path.split(['/', '\\']) {
        if segment.is_empty() || segment == "." || segment == ".." {
            continue;
        }
        if is_private_segment(segment) {
            return true;
        }
        if engine == SearchEngine::Plain && is_plain_skip_segment(segment) {
            return true;
        }
    }
    false
}

fn is_private_segment(segment: &str) -> bool {
    let lower = segment.to_lowercase();
    let has_secret_ext = Path::new(segment)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("pem") || ext.eq_ignore_ascii_case("key"));
    PRIVATE_EXACT_SEGMENTS.contains(&lower.as_str())
        || PRIVATE_PREFIX_SEGMENTS
            .iter()
            .any(|prefix| lower.starts_with(prefix))
        || has_secret_ext
}

fn is_plain_skip_segment(segment: &str) -> bool {
    PLAIN_ONLY_SKIP_SEGMENTS
        .iter()
        .any(|skip| segment.eq_ignore_ascii_case(skip))
}

#[must_use]
pub fn clip_line(text: &str) -> String {
    text.chars().take(SEARCH_LINE_CHARS).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_clips_long_lines() {
        let long = "a".repeat(500);
        let clipped = clip_line(&long);
        assert_eq!(clipped.chars().count(), SEARCH_LINE_CHARS);
        let short = "hello";
        assert_eq!(clip_line(short), short);
    }

    #[test]
    fn plain_search_skips_private() {
        assert!(skipped(SearchEngine::Ripgrep, ".git/HEAD"));
        assert!(skipped(SearchEngine::Ripgrep, ".env"));
        assert!(skipped(SearchEngine::Ripgrep, "secrets/.env.local"));
        assert!(skipped(SearchEngine::Ripgrep, "certs/dev.pem"));
        assert!(skipped(SearchEngine::Ripgrep, "certs/priv.key"));
        assert!(!skipped(SearchEngine::Ripgrep, "node_modules/x.js"));
        assert!(!skipped(SearchEngine::Ripgrep, "target/debug/out"));

        assert!(skipped(SearchEngine::Plain, ".git/HEAD"));
        assert!(skipped(SearchEngine::Plain, ".env"));
        assert!(skipped(SearchEngine::Plain, "certs/dev.pem"));
        assert!(skipped(SearchEngine::Plain, "node_modules/x.js"));
        assert!(skipped(SearchEngine::Plain, "target/debug/out"));
        assert!(!skipped(SearchEngine::Plain, "src/main.rs"));
    }
}
