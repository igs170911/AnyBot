use unicode_normalization::UnicodeNormalization;

#[must_use]
pub(crate) fn nfkc(text: &str) -> String {
    text.nfkc().collect()
}

#[must_use]
pub(crate) fn is_cjk(ch: char) -> bool {
    matches!(
        ch,
        '\u{3400}'..='\u{4DBF}'
            | '\u{4E00}'..='\u{9FFF}'
            | '\u{F900}'..='\u{FAFF}'
            | '\u{20000}'..='\u{2A6DF}'
            | '\u{2A700}'..='\u{2B73F}'
            | '\u{2B740}'..='\u{2B81F}'
            | '\u{2B820}'..='\u{2CEAF}'
            | '\u{2CEB0}'..='\u{2EBEF}'
            | '\u{30000}'..='\u{3134F}'
    )
}

#[must_use]
pub(crate) fn is_ident_connector(ch: char) -> bool {
    matches!(ch, '_' | '-' | '.' | '/')
}

#[must_use]
pub(crate) fn is_ident_body(ch: char) -> bool {
    (ch.is_alphanumeric() && !is_cjk(ch)) || is_ident_connector(ch)
}

#[must_use]
pub(crate) fn cjk_runs(text: &str) -> Vec<Vec<char>> {
    let mut runs = Vec::new();
    let mut current = Vec::new();
    for ch in text.chars() {
        if is_cjk(ch) {
            current.push(ch);
        } else if !current.is_empty() {
            runs.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        runs.push(current);
    }
    runs
}

#[must_use]
pub(crate) fn ident_tokens(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        if is_ident_body(ch) {
            current.push(ch);
        } else {
            flush_ident(&mut current, &mut tokens);
        }
    }
    flush_ident(&mut current, &mut tokens);
    tokens
}

fn flush_ident(buf: &mut String, out: &mut Vec<String>) {
    if buf.is_empty() {
        return;
    }
    let raw = std::mem::take(buf);
    let trimmed = raw.trim_matches(is_ident_connector);
    if trimmed.chars().any(char::is_alphanumeric) {
        out.push(trimmed.to_owned());
    }
}

#[must_use]
pub(crate) fn split_ident_parts(raw: &str) -> Vec<String> {
    let chars: Vec<char> = raw.chars().collect();
    let mut parts = Vec::new();
    let mut current = String::new();
    for (idx, &ch) in chars.iter().enumerate() {
        if is_ident_connector(ch) {
            push_lower_part(&mut current, &mut parts);
            continue;
        }
        let prev = idx.checked_sub(1).and_then(|i| chars.get(i).copied());
        let next = chars.get(idx.saturating_add(1)).copied();
        if let Some(p) = prev
            && !is_ident_connector(p)
            && is_part_boundary(p, ch, next)
        {
            push_lower_part(&mut current, &mut parts);
        }
        current.push(ch);
    }
    push_lower_part(&mut current, &mut parts);
    parts
}

fn push_lower_part(buf: &mut String, out: &mut Vec<String>) {
    if !buf.is_empty() {
        let piece = std::mem::take(buf).to_lowercase();
        if !piece.is_empty() {
            out.push(piece);
        }
    }
}

fn is_part_boundary(prev: char, curr: char, next: Option<char>) -> bool {
    if prev.is_numeric() != curr.is_numeric() {
        return true;
    }
    if prev.is_lowercase() && curr.is_uppercase() {
        return true;
    }
    prev.is_uppercase() && curr.is_uppercase() && next.is_some_and(char::is_lowercase)
}

#[must_use]
pub(crate) fn is_uuid_shape(token: &str) -> bool {
    let bytes = token.as_bytes();
    if bytes.len() != 36 {
        return false;
    }
    for (idx, &byte) in bytes.iter().enumerate() {
        if matches!(idx, 8 | 13 | 18 | 23) {
            if byte != b'-' {
                return false;
            }
        } else if !byte.is_ascii_hexdigit() {
            return false;
        }
    }
    true
}
