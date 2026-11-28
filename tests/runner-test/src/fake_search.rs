use std::collections::BTreeSet;
use std::path::Path;
use std::time::Duration;

use anybot_protocol::{SearchCut, SearchEngine, SearchHit, SearchOutcome, SearchRequest};
use anybot_trust::trust_floor::is_private_path;
use test_harness::FakeClock;

use crate::fake_files::FileTree;

pub const SEARCH_MAX_MS: u64 = 25_000;
pub const SEARCH_MAX_LINES: usize = 10_000;
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

enum Matcher {
    Regex(regex::Regex),
    Plain(String),
}

impl Matcher {
    fn new(engine: SearchEngine, pattern: &str) -> Result<Self, String> {
        match engine {
            SearchEngine::Ripgrep => {
                let re = regex::Regex::new(pattern).map_err(|e| format!("invalid regex: {e}"))?;
                Ok(Matcher::Regex(re))
            }
            SearchEngine::Plain => Ok(Matcher::Plain(pattern.to_lowercase())),
        }
    }

    fn matches(&self, line: &str) -> bool {
        match self {
            Matcher::Regex(re) => re.is_match(line),
            Matcher::Plain(needle) => line.to_lowercase().contains(needle),
        }
    }
}

#[must_use]
pub fn skipped(engine: SearchEngine, path: &str) -> bool {
    if is_private_path(path) || is_private_path(&format!("~/{}", path.trim_start_matches('/'))) {
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

struct SearchAcc {
    hits: Vec<SearchHit>,
    seen_files: BTreeSet<String>,
    files: Vec<String>,
    total_matches: usize,
    cut: SearchCut,
}

impl SearchAcc {
    fn new() -> Self {
        Self {
            hits: Vec::new(),
            seen_files: BTreeSet::new(),
            files: Vec::new(),
            total_matches: 0,
            cut: SearchCut::None,
        }
    }

    fn scan_file(&mut self, rel_path: &str, content: &str, matcher: &Matcher) -> bool {
        for (idx, line) in content.lines().enumerate() {
            if !matcher.matches(line) {
                continue;
            }
            if self.total_matches >= SEARCH_MAX_LINES {
                self.cut = SearchCut::MaxLines;
                return true;
            }
            self.total_matches = self.total_matches.saturating_add(1);
            let line_no = u32::try_from(idx.saturating_add(1)).unwrap_or(u32::MAX);
            self.hits.push(SearchHit {
                path: rel_path.to_owned(),
                line: line_no,
                text: clip_line(line),
            });
            if self.seen_files.insert(rel_path.to_owned()) {
                self.files.push(rel_path.to_owned());
            }
        }
        false
    }
}

pub fn run_search(
    tree: &FileTree,
    clock: &FakeClock,
    engine: SearchEngine,
    query: &SearchRequest,
    per_file_ms: u64,
) -> Result<SearchOutcome, String> {
    let matcher = Matcher::new(engine, &query.pattern)?;
    let candidates = tree.search_files(&query.path)?;
    let started = clock.now_ms();
    let mut acc = SearchAcc::new();

    for (rel_path, content) in candidates {
        if skipped(engine, &rel_path) {
            continue;
        }
        if per_file_ms > 0 {
            let _wakes = clock.advance(Duration::from_millis(per_file_ms));
            if clock.now_ms().saturating_sub(started) > SEARCH_MAX_MS {
                acc.cut = SearchCut::MaxTime;
                break;
            }
        }
        if acc.scan_file(&rel_path, &content, &matcher) {
            break;
        }
    }

    Ok(SearchOutcome {
        engine,
        hits: if query.files_only {
            Vec::new()
        } else {
            acc.hits
        },
        files: acc.files,
        cut: acc.cut,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_clips_long_lines() {
        let mut tree = FileTree::new();
        tree.write("a.txt", &format!("prefix-{}", "x".repeat(500)));
        let clock = FakeClock::new(0);
        let outcome = run_search(
            &tree,
            &clock,
            SearchEngine::Ripgrep,
            &SearchRequest {
                pattern: "prefix".to_owned(),
                path: ".".to_owned(),
                files_only: false,
            },
            0,
        )
        .unwrap();
        assert_eq!(outcome.hits.len(), 1);
        assert_eq!(
            outcome.hits.first().map(|hit| hit.text.chars().count()),
            Some(SEARCH_LINE_CHARS)
        );
    }

    #[test]
    fn search_stops_at_limits() {
        use std::fmt::Write as _;

        let mut many_tree = FileTree::new();
        let mut body = String::new();
        for i in 1..=12_000 {
            writeln!(&mut body, "hit-{i}").unwrap();
        }
        many_tree.write("big.txt", &body);
        let clock = FakeClock::new(0);
        let max_lines = run_search(
            &many_tree,
            &clock,
            SearchEngine::Ripgrep,
            &SearchRequest {
                pattern: "hit-".to_owned(),
                path: ".".to_owned(),
                files_only: false,
            },
            0,
        )
        .unwrap();
        assert_eq!(max_lines.cut, SearchCut::MaxLines);
        assert_eq!(max_lines.hits.len(), SEARCH_MAX_LINES);

        let mut slow_tree = FileTree::new();
        for i in 0..6 {
            slow_tree.write(&format!("f{i}.txt"), "match\n");
        }
        let slow_clock = FakeClock::new(0);
        let timed_out = run_search(
            &slow_tree,
            &slow_clock,
            SearchEngine::Plain,
            &SearchRequest {
                pattern: "match".to_owned(),
                path: ".".to_owned(),
                files_only: false,
            },
            5_001,
        )
        .unwrap();
        assert_eq!(timed_out.cut, SearchCut::MaxTime);
        assert_eq!(slow_clock.now_ms(), 25_005);
    }

    #[test]
    fn search_lists_files_only() {
        let mut tree = FileTree::new();
        tree.write("a.txt", "hit\nhit\n");
        tree.write("b.txt", "hit\n");
        tree.write("c.txt", "other\n");
        let clock = FakeClock::new(0);
        let outcome = run_search(
            &tree,
            &clock,
            SearchEngine::Plain,
            &SearchRequest {
                pattern: "HIT".to_owned(),
                path: ".".to_owned(),
                files_only: true,
            },
            0,
        )
        .unwrap();
        assert!(outcome.hits.is_empty());
        assert_eq!(outcome.files, vec!["a.txt".to_owned(), "b.txt".to_owned()]);
    }
}
