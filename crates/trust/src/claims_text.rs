#[must_use]
pub(crate) fn strip_code_and_quotes(text: &str) -> String {
    let no_code = strip_code_spans(text);
    strip_quoted_spans(&no_code)
}

#[must_use]
pub(crate) fn strip_code_spans(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut idx = 0usize;
    while idx < text.len() {
        let Some(rest) = text.get(idx..) else {
            break;
        };
        if let Some(after) = rest.strip_prefix("```") {
            if let Some(end) = after.find("```") {
                out.push(' ');
                idx = idx.saturating_add(3).saturating_add(end).saturating_add(3);
                continue;
            }
            break;
        }
        if let Some(after) = rest.strip_prefix('`') {
            if let Some(end) = after.find('`') {
                out.push(' ');
                idx = idx.saturating_add(1).saturating_add(end).saturating_add(1);
                continue;
            }
            break;
        }
        let Some(ch) = rest.chars().next() else {
            break;
        };
        out.push(ch);
        idx = idx.saturating_add(ch.len_utf8());
    }
    out
}

fn strip_quoted_spans(text: &str) -> String {
    let pairs: [(char, char); 4] = [
        ('"', '"'),
        ('\u{300c}', '\u{300d}'),
        ('\u{300e}', '\u{300f}'),
        ('\u{201c}', '\u{201d}'),
    ];
    let mut out = String::with_capacity(text.len());
    let mut idx = 0usize;
    while idx < text.len() {
        let Some(rest) = text.get(idx..) else {
            break;
        };
        let Some(ch) = rest.chars().next() else {
            break;
        };
        if let Some((_, close)) = pairs.iter().find(|(open, _)| *open == ch) {
            let inner = rest.get(ch.len_utf8()..).unwrap_or("");
            if let Some(end) = inner.find(*close) {
                out.push(' ');
                idx = idx
                    .saturating_add(ch.len_utf8())
                    .saturating_add(end)
                    .saturating_add(close.len_utf8());
                continue;
            }
        }
        out.push(ch);
        idx = idx.saturating_add(ch.len_utf8());
    }
    out
}

#[must_use]
pub(crate) fn extract_claim_counts(sentence: &str) -> (Option<u32>, Option<u32>) {
    let lower = sentence.to_lowercase();
    let mut said_sends = None;
    let mut said_files = None;
    for (num, unit) in scan_number_units(&lower) {
        if matches!(
            unit.as_str(),
            "\u{5247}"
                | "\u{689d}"
                | "\u{5c01}"
                | "\u{7b46}"
                | "\u{6b21}"
                | "message"
                | "messages"
                | "email"
                | "emails"
        ) {
            said_sends = Some(num);
        }
        if matches!(
            unit.as_str(),
            "\u{5f35}"
                | "\u{500b}"
                | "\u{4efd}"
                | "file"
                | "files"
                | "image"
                | "images"
                | "photo"
                | "photos"
        ) {
            said_files = Some(num);
        }
    }
    (said_sends, said_files)
}

fn scan_number_units(text: &str) -> Vec<(u32, String)> {
    let mut out = Vec::new();
    let mut idx = 0usize;
    while idx < text.len() {
        let Some(rest) = text.get(idx..) else {
            break;
        };
        let Some(ch) = rest.chars().next() else {
            break;
        };
        if ch.is_ascii_digit() {
            let d_len = rest
                .chars()
                .take_while(char::is_ascii_digit)
                .map(char::len_utf8)
                .sum();
            if let Some(num_str) = rest.get(..d_len)
                && let Ok(val) = num_str.parse::<u32>()
            {
                let after_num = rest.get(d_len..).unwrap_or("").trim_start();
                let unit = take_unit(after_num);
                if !unit.is_empty() {
                    out.push((val, unit));
                }
            }
            idx = idx.saturating_add(d_len);
            continue;
        }
        idx = idx.saturating_add(ch.len_utf8());
    }
    out
}

fn take_unit(text: &str) -> String {
    let Some(first) = text.chars().next() else {
        return String::new();
    };
    if first.is_ascii_alphabetic() {
        return text.chars().take_while(char::is_ascii_alphabetic).collect();
    }
    first.to_string()
}
