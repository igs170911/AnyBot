use std::collections::BTreeMap;

pub const CJK_RANGES: [(u32, u32); 16] = [
    (0x1100, 0x11FF),
    (0x2E80, 0x2FDF),
    (0x3000, 0x30FF),
    (0x3130, 0x318F),
    (0x31C0, 0x31EF),
    (0x3200, 0x33FF),
    (0x3400, 0x4DBF),
    (0x4E00, 0x9FFF),
    (0xA960, 0xD7FF),
    (0xF900, 0xFAFF),
    (0xFE30, 0xFE4F),
    (0xFF00, 0xFF60),
    (0xFF66, 0xFFDC),
    (0x20000, 0x2A6DF),
    (0x2A700, 0x2EBEF),
    (0x2F800, 0x2FA1F),
];

#[must_use]
pub fn is_cjk(ch: char) -> bool {
    let cp = u32::from(ch);
    CJK_RANGES.iter().any(|(lo, hi)| cp >= *lo && cp <= *hi)
}

#[must_use]
pub fn cjk_violations(files: &[(String, String)]) -> Vec<String> {
    let mut out = Vec::new();
    for (path, content) in files {
        let mut seen: BTreeMap<char, usize> = BTreeMap::new();
        for (line_no, line) in content.lines().enumerate() {
            for ch in line.chars() {
                if is_cjk(ch) {
                    let first_line = line_no.checked_add(1).unwrap_or_default();
                    seen.entry(ch).or_insert(first_line);
                }
            }
        }
        for (ch, line_no) in seen {
            out.push(format!(
                "{path}:{line_no}: CJK codepoint U+{:04X} is not allowed outside locales (spec/09 K91)",
                u32::from(ch),
            ));
        }
    }
    out
}

#[must_use]
pub fn is_excluded_path(path: &str) -> bool {
    const EXCLUDED_PREFIXES: [&str; 4] = ["locales/", "spec/", "docs/", "tests/fixtures/cjk/"];
    EXCLUDED_PREFIXES.iter().any(|p| path.starts_with(p))
        || path == "AGENTS.md"
        || path == "pnpm-lock.yaml"
        || path == "Cargo.lock"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blocked(files: &[(String, String)]) -> bool {
        let got = cjk_violations(files);
        !got.is_empty() && got.first().is_some_and(|v| v.contains("U+"))
    }

    fn one(path: &str, text: &str) -> Vec<(String, String)> {
        vec![(path.to_owned(), text.to_owned())]
    }

    #[test]
    fn test_cjk_violations_block_han_hiragana_katakana_hangul() {
        let han = "\u{4e2d}\u{6587}";
        let hiragana = "\u{307f}\u{3082}";
        assert!(blocked(&one("crates/x/src/a.rs", han)));
        assert!(blocked(&one("crates/x/src/a.rs", hiragana)));
        let katakana = "\u{30ab}\u{30bf}\u{30ab}\u{30ca}";
        let hangul = "\u{d55c}\u{ad6d}\u{c5b4}";
        assert!(blocked(&one("crates/x/src/a.rs", katakana)));
        assert!(blocked(&one("crates/x/src/a.rs", hangul)));
    }

    #[test]
    fn test_cjk_violations_block_fullwidth_punctuation_ext_b_compat() {
        let fullwidth = "\u{ff21}\u{ff22}";
        let punctuation = "\u{300c}\u{300d}";
        let ext_b = "\u{20000}";
        let compat = "\u{f900}";
        assert!(blocked(&one("web/ui/src/a.tsx", fullwidth)));
        assert!(blocked(&one("web/ui/src/a.tsx", punctuation)));
        assert!(blocked(&one("web/ui/src/a.tsx", ext_b)));
        assert!(blocked(&one("web/ui/src/a.tsx", compat)));
    }

    #[test]
    fn test_cjk_violations_pass_ascii_and_accented_latin() {
        let text = "ASCII text with dashes - and the accent \\u{301} as escape.";
        let got = cjk_violations(&one("apps/server/src/main.rs", text));
        assert!(got.is_empty());
    }

    #[test]
    fn test_is_cjk_covers_declared_ranges() {
        assert!(is_cjk('\u{4e2d}'));
        assert!(is_cjk('\u{3000}'));
        assert!(is_cjk('\u{ff5f}'));
        assert!(is_cjk('\u{1100}'));
        assert!(is_cjk('\u{20000}'));
        assert!(is_cjk('\u{2ebf}'));
        assert!(is_cjk('\u{2ec0}'));
        assert!(is_cjk('\u{31ef}'));
        assert!(is_cjk('\u{2a6df}'));
    }

    #[test]
    fn test_is_cjk_excludes_adjacent_non_cjk_blocks() {
        assert!(!is_cjk('\u{2014}'));
        assert!(!is_cjk('\u{00e9}'));
        assert!(!is_cjk('\u{0416}'));
        assert!(!is_cjk('\u{1f600}'));
        assert!(!is_cjk('\u{2fe0}'));
        assert!(!is_cjk('\u{ff61}'));
        assert!(!is_cjk('\u{2a6e0}'));
    }

    #[test]
    fn test_excluded_paths_cover_locales_spec_docs_agents() {
        assert!(is_excluded_path("locales/en.ftl"));
        assert!(is_excluded_path("spec/09-decisions.md"));
        assert!(is_excluded_path("docs/plan.md"));
        assert!(is_excluded_path("AGENTS.md"));
        assert!(is_excluded_path("pnpm-lock.yaml"));
        assert!(!is_excluded_path("crates/messages/src/catalog.rs"));
        assert!(!is_excluded_path("README.md"));
        assert!(!is_excluded_path("Makefile"));
    }
}
