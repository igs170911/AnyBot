use anybot_protocol::EvidenceShortId;

use crate::claims_phrases::{has_done_claim, has_send_claim, is_non_claim_exclusion};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExpectedKind {
    Send,
    Change,
    Execute,
}

impl ExpectedKind {
    #[must_use]
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            ExpectedKind::Send => "send",
            ExpectedKind::Change => "change",
            ExpectedKind::Execute => "execute",
        }
    }
}

const SEND_STEMS: [&str; 17] = [
    "\u{5bc4}\u{51fa}",
    "\u{5bc4}\u{7d66}",
    "\u{9001}\u{51fa}",
    "\u{9001}\u{7d66}",
    "\u{50b3}\u{9001}",
    "\u{50b3}\u{7d66}",
    "\u{767c}\u{4f48}",
    "\u{767c}\u{5e03}",
    "\u{4e0a}\u{50b3}",
    "\u{767c}\u{9001}",
    "\u{5206}\u{4eab}",
    "sent ",
    "emailed ",
    "mailed ",
    "uploaded ",
    "published ",
    "delivered ",
];

const CHANGE_STEMS: [&str; 17] = [
    "\u{6539}\u{4e86}",
    "\u{4fee}\u{6539}",
    "\u{6539}\u{597d}",
    "\u{5beb}\u{5165}",
    "\u{5beb}\u{597d}",
    "\u{5efa}\u{7acb}",
    "\u{5efa}\u{597d}",
    "\u{522a}\u{9664}",
    "\u{6539}\u{6a94}",
    "edited ",
    "modified ",
    "wrote ",
    "written ",
    "created ",
    "updated ",
    "changed ",
    "deleted ",
];

const EXECUTE_STEMS: [&str; 12] = [
    "\u{8dd1}\u{904e}",
    "\u{57f7}\u{884c}\u{904e}",
    "\u{57f7}\u{884c}\u{4e86}",
    "\u{6e2c}\u{904e}",
    "\u{6e2c}\u{8a66}\u{904e}",
    "\u{6e2c}\u{8a66}\u{901a}\u{904e}",
    "\u{8dd1}\u{4e86}",
    "ran ",
    "executed ",
    "tested ",
    "tests passed",
    "command succeeded",
];

#[must_use]
pub(crate) fn split_report_sentences(reply: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut iter = reply.chars().peekable();
    while let Some(ch) = iter.next() {
        if matches!(ch, '\n' | '\u{3002}' | '\u{ff01}' | '\u{ff1f}' | '!' | '?') {
            push_trimmed(&mut out, &mut cur);
            continue;
        }
        if ch == '.' && iter.peek().is_none_or(|next| next.is_whitespace()) {
            push_trimmed(&mut out, &mut cur);
            continue;
        }
        cur.push(ch);
    }
    push_trimmed(&mut out, &mut cur);
    out
}

fn push_trimmed(out: &mut Vec<String>, cur: &mut String) {
    let trimmed = cur.trim();
    if !trimmed.is_empty() {
        out.push(trimmed.to_owned());
    }
    cur.clear();
}

#[must_use]
pub(crate) fn extract_citations(sentence: &str) -> Vec<EvidenceShortId> {
    let mut out = Vec::new();
    scan_bracket_pair(sentence, '\u{3014}', '\u{3015}', &mut out);
    scan_bracket_pair(sentence, '[', ']', &mut out);
    out
}

fn scan_bracket_pair(sentence: &str, open: char, close: char, out: &mut Vec<EvidenceShortId>) {
    let mut idx = 0usize;
    while idx < sentence.len() {
        let Some(rest) = sentence.get(idx..) else {
            break;
        };
        let Some(open_pos) = rest.find(open) else {
            break;
        };
        let after_open = idx.saturating_add(open_pos).saturating_add(open.len_utf8());
        let Some(tail) = sentence.get(after_open..) else {
            break;
        };
        let Some(close_pos) = tail.find(close) else {
            break;
        };
        if let Some(inner) = tail.get(..close_pos) {
            parse_citation_group(inner, out);
        }
        idx = after_open
            .saturating_add(close_pos)
            .saturating_add(close.len_utf8());
    }
}

fn parse_citation_group(inner: &str, out: &mut Vec<EvidenceShortId>) {
    for part in inner.split(['\u{3001}', ',', ';', ' ', '\t']) {
        let trimmed = part.trim();
        let Some(digits) = trimmed.strip_prefix(['E', 'e']) else {
            continue;
        };
        if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        if let Ok(number) = digits.parse::<u32>() {
            let short = EvidenceShortId(number);
            if !out.contains(&short) {
                out.push(short);
            }
        }
    }
}

#[must_use]
pub(crate) fn strip_citations(sentence: &str) -> String {
    let mut out = String::with_capacity(sentence.len());
    let mut idx = 0usize;
    while idx < sentence.len() {
        let Some(rest) = sentence.get(idx..) else {
            break;
        };
        let Some(ch) = rest.chars().next() else {
            break;
        };
        if let Some(close) = match ch {
            '\u{3014}' => Some('\u{3015}'),
            '[' => Some(']'),
            _ => None,
        } && let Some(inner) = rest.get(ch.len_utf8()..)
            && let Some(end) = inner.find(close)
        {
            out.push(' ');
            idx = idx
                .saturating_add(ch.len_utf8())
                .saturating_add(end)
                .saturating_add(close.len_utf8());
            continue;
        }
        out.push(ch);
        idx = idx.saturating_add(ch.len_utf8());
    }
    out
}

#[must_use]
pub(crate) fn is_report_claim(sentence: &str, cited: &[EvidenceShortId]) -> bool {
    if !cited.is_empty() {
        return true;
    }
    let clean = strip_citations(sentence);
    if is_non_claim_exclusion(&clean) {
        return false;
    }
    has_done_claim(&clean) || has_send_claim(&clean) || expected_kind(&clean).is_some()
}

#[must_use]
pub(crate) fn expected_kind(sentence: &str) -> Option<ExpectedKind> {
    let clean = strip_citations(sentence);
    let lower = clean.to_lowercase();
    if has_send_claim(&clean) || SEND_STEMS.iter().any(|stem| lower.contains(stem)) {
        return Some(ExpectedKind::Send);
    }
    if CHANGE_STEMS.iter().any(|stem| lower.contains(stem)) {
        return Some(ExpectedKind::Change);
    }
    if EXECUTE_STEMS.iter().any(|stem| lower.contains(stem)) {
        return Some(ExpectedKind::Execute);
    }
    None
}
