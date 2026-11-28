use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

use crate::memory_secret_rules::find_secret_patterns;

pub const CODE_MEMORY_SCAN_INJECTION: &str = "memory-scan-injection";
pub const CODE_MEMORY_SCAN_SECRET: &str = "memory-scan-secret";
pub const CODE_MEMORY_SCAN_INVISIBLE: &str = "memory-scan-invisible";

pub const INJECTION_PATTERNS: [(&str, &str); 17] = [
    ("ignore-previous", "ignore previous instructions"),
    ("ignore-all-previous", "ignore all previous instructions"),
    ("disregard-previous", "disregard previous instructions"),
    ("forget-previous", "forget all previous instructions"),
    ("override-system", "override system prompt"),
    ("new-system-prompt", "new system prompt"),
    ("reveal-system-prompt", "reveal your system prompt"),
    ("system-prompt-colon", "system prompt:"),
    ("developer-mode", "enable developer mode"),
    ("tag-system", "<|system|>"),
    ("tag-im-start-system", "<|im_start|>system"),
    ("tag-sys-block", "<<sys>>"),
    (
        "zh-ignore-previous-instructions",
        "\u{5ffd}\u{7565}\u{4e4b}\u{524d}\u{7684}\u{6307}\u{793a}",
    ),
    (
        "zh-ignore-above-commands",
        "\u{5ffd}\u{7565}\u{4ee5}\u{4e0a}\u{6307}\u{4ee4}",
    ),
    (
        "zh-ignore-prior-commands",
        "\u{5ffd}\u{7565}\u{5148}\u{524d}\u{7684}\u{6307}\u{4ee4}",
    ),
    (
        "zh-disregard-prior-rules",
        "\u{7121}\u{8996}\u{4e4b}\u{524d}\u{7684}\u{898f}\u{5247}",
    ),
    (
        "zh-change-system-prompt",
        "\u{7cfb}\u{7d71}\u{63d0}\u{793a}\u{8a5e}\u{6539}\u{6210}",
    ),
];

pub const INVISIBLE_CHARS: [(char, &str); 13] = [
    ('\u{00AD}', "soft-hyphen"),
    ('\u{200B}', "zero-width-space"),
    ('\u{200C}', "zero-width-non-joiner"),
    ('\u{200D}', "zero-width-joiner"),
    ('\u{200E}', "left-to-right-mark"),
    ('\u{200F}', "right-to-left-mark"),
    ('\u{202A}', "bidi-lre"),
    ('\u{202B}', "bidi-rle"),
    ('\u{202C}', "bidi-pdf"),
    ('\u{202D}', "bidi-lro"),
    ('\u{202E}', "bidi-rlo"),
    ('\u{2060}', "word-joiner"),
    ('\u{FEFF}', "bom-zwnbsp"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanCategory {
    PromptInjection,
    Secret,
    InvisibleChar,
}

impl ScanCategory {
    #[must_use]
    pub fn error_code(self) -> &'static str {
        match self {
            ScanCategory::PromptInjection => CODE_MEMORY_SCAN_INJECTION,
            ScanCategory::Secret => CODE_MEMORY_SCAN_SECRET,
            ScanCategory::InvisibleChar => CODE_MEMORY_SCAN_INVISIBLE,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanFinding {
    pub category: ScanCategory,
    pub pattern: String,
    pub offset: usize,
}

#[must_use]
pub fn scan(text: &str) -> Vec<ScanFinding> {
    let mut findings = Vec::new();
    scan_invisible(text, &mut findings);
    scan_secrets(text, &mut findings);
    scan_injections(text, &mut findings);
    findings
}

fn scan_invisible(text: &str, out: &mut Vec<ScanFinding>) {
    for (offset, ch) in text.char_indices() {
        if let Some((_, label)) = INVISIBLE_CHARS.iter().find(|&&(bad, _)| bad == ch) {
            out.push(ScanFinding {
                category: ScanCategory::InvisibleChar,
                pattern: (*label).to_owned(),
                offset,
            });
            continue;
        }
        if matches!(ch, '\u{2066}'..='\u{2069}' | '\u{E0000}'..='\u{E007F}') {
            out.push(ScanFinding {
                category: ScanCategory::InvisibleChar,
                pattern: "unicode-format-tag".to_owned(),
                offset,
            });
            continue;
        }
        if ch.is_control() && !matches!(ch, '\n' | '\r' | '\t') {
            out.push(ScanFinding {
                category: ScanCategory::InvisibleChar,
                pattern: "control-char".to_owned(),
                offset,
            });
        }
    }
}

fn scan_secrets(text: &str, out: &mut Vec<ScanFinding>) {
    for item in find_secret_patterns(text) {
        out.push(ScanFinding {
            category: ScanCategory::Secret,
            pattern: item.pattern.to_owned(),
            offset: item.start,
        });
    }
}

fn scan_injections(text: &str, out: &mut Vec<ScanFinding>) {
    let normalized = collapse_nfkc_lower(text);
    for &(label, phrase) in &INJECTION_PATTERNS {
        if let Some(offset) = normalized.find(phrase) {
            out.push(ScanFinding {
                category: ScanCategory::PromptInjection,
                pattern: label.to_owned(),
                offset,
            });
        }
    }
}

fn collapse_nfkc_lower(text: &str) -> String {
    let nfkc: String = text.nfkc().collect();
    let lower = nfkc.to_lowercase();
    let mut collapsed = String::with_capacity(lower.len());
    let mut last_space = true;
    for ch in lower.chars() {
        if ch.is_whitespace() {
            if !last_space {
                collapsed.push(' ');
                last_space = true;
            }
        } else {
            collapsed.push(ch);
            last_space = false;
        }
    }
    if collapsed.ends_with(' ') {
        collapsed.pop();
    }
    collapsed
}
