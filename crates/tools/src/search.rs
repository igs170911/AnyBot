use std::collections::BTreeSet;

use anybot_protocol::{SearchCut, SearchOutcome};

use crate::message_codes::{
    CODE_TOOL_SEARCH_CUT, CODE_TOOL_SEARCH_EMPTY, CODE_TOOL_SEARCH_TIMED_OUT,
};

pub const SEARCH_RETURN_LINES: usize = 2_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchText {
    pub lines: Vec<String>,
    pub found: usize,
    pub cut: SearchCut,
    pub is_empty: bool,
}

#[must_use]
pub fn search_cut_code() -> &'static str {
    CODE_TOOL_SEARCH_CUT
}

#[must_use]
pub fn search_empty_code() -> &'static str {
    CODE_TOOL_SEARCH_EMPTY
}

#[must_use]
pub fn search_timed_out_code() -> &'static str {
    CODE_TOOL_SEARCH_TIMED_OUT
}

#[must_use]
pub fn render_search(outcome: &SearchOutcome, files_only: bool) -> SearchText {
    if files_only {
        let files = collect_files(outcome);
        let found = files.len();
        let lines: Vec<String> = files.into_iter().take(SEARCH_RETURN_LINES).collect();
        let is_empty = lines.is_empty();
        return SearchText {
            lines,
            found,
            cut: outcome.cut,
            is_empty,
        };
    }
    let found = outcome.hits.len();
    let lines: Vec<String> = outcome
        .hits
        .iter()
        .take(SEARCH_RETURN_LINES)
        .map(|hit| format!("{}:{}:{}", hit.path, hit.line, hit.text))
        .collect();
    let is_empty = lines.is_empty();
    SearchText {
        lines,
        found,
        cut: outcome.cut,
        is_empty,
    }
}

fn collect_files(outcome: &SearchOutcome) -> Vec<String> {
    if !outcome.files.is_empty() {
        return outcome.files.clone();
    }
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for hit in &outcome.hits {
        if seen.insert(hit.path.clone()) {
            out.push(hit.path.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use anybot_protocol::{SearchEngine, SearchHit};

    fn make_hit(path: &str, line: u32, text: &str) -> SearchHit {
        SearchHit {
            path: path.to_owned(),
            line,
            text: text.to_owned(),
        }
    }

    #[test]
    fn search_lists_files_only() {
        let outcome = SearchOutcome {
            engine: SearchEngine::Ripgrep,
            hits: vec![
                make_hit("a.txt", 1, "foo"),
                make_hit("a.txt", 3, "foo again"),
                make_hit("b.txt", 2, "foo"),
            ],
            files: vec!["a.txt".to_owned(), "b.txt".to_owned()],
            cut: SearchCut::None,
        };
        let rendered = render_search(&outcome, true);
        assert_eq!(rendered.lines, vec!["a.txt".to_owned(), "b.txt".to_owned()]);
        assert_eq!(rendered.found, 2);
        assert!(!rendered.is_empty);

        let from_hits_only = SearchOutcome {
            engine: SearchEngine::Plain,
            hits: outcome.hits,
            files: Vec::new(),
            cut: SearchCut::None,
        };
        let deduped = render_search(&from_hits_only, true);
        assert_eq!(deduped.lines, vec!["a.txt".to_owned(), "b.txt".to_owned()]);
    }

    #[test]
    fn search_caps_returned_lines_and_keeps_found_count() {
        let hits: Vec<SearchHit> = (1_u32..=2_500)
            .map(|n| make_hit("a.txt", n, "match"))
            .collect();
        let outcome = SearchOutcome {
            engine: SearchEngine::Ripgrep,
            hits,
            files: Vec::new(),
            cut: SearchCut::MaxLines,
        };
        let rendered = render_search(&outcome, false);
        assert_eq!(rendered.lines.len(), SEARCH_RETURN_LINES);
        assert_eq!(rendered.found, 2_500);
        assert_eq!(rendered.cut, SearchCut::MaxLines);
    }
}
