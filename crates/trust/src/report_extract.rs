use crate::report_sentences::strip_citations;

#[must_use]
pub(crate) fn extract_emails(text: &str) -> Vec<String> {
    let clean = strip_citations(text);
    let mut out = Vec::new();
    for segment in clean.split(|c: char| !is_email_char(c)) {
        let trimmed = segment.trim_matches(['.', '-', '_']);
        if is_valid_email(trimmed) {
            let lower = trimmed.to_lowercase();
            if !out.contains(&lower) {
                out.push(lower);
            }
        }
    }
    out
}

fn is_email_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '@' | '.' | '_' | '+' | '-')
}

fn is_valid_email(text: &str) -> bool {
    let Some((local, domain)) = text.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && !domain.is_empty()
        && !domain.contains('@')
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
}

#[must_use]
pub(crate) fn extract_recipients(sentence: &str) -> Vec<String> {
    let emails = extract_emails(sentence);
    if !emails.is_empty() {
        return emails;
    }
    let clean = strip_citations(sentence);
    let lower = clean.to_lowercase();
    let mut out = Vec::new();
    for marker in [
        "\u{5bc4}\u{7d66}",
        "\u{9001}\u{7d66}",
        "\u{50b3}\u{7d66}",
        "sent to ",
    ] {
        if let Some((_, after)) = lower.split_once(marker)
            && let Some(token) = after.split_whitespace().next()
        {
            let name =
                token.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-');
            if !name.is_empty() && !name.contains('.') && !is_pronoun(name) {
                out.push(name.to_owned());
            }
        }
    }
    out
}

fn is_pronoun(token: &str) -> bool {
    matches!(token, "you" | "me" | "him" | "her" | "us" | "them" | "user")
}

#[must_use]
pub(crate) fn extract_urls(text: &str) -> Vec<String> {
    let clean = strip_citations(text);
    let mut out = Vec::new();
    for token in clean.split_whitespace() {
        let trimmed = token
            .trim_matches(|c: char| matches!(c, '(' | ')' | '[' | ']' | '<' | '>' | ',' | ';'));
        let lower = trimmed.to_lowercase();
        if lower.starts_with("http://")
            || lower.starts_with("https://")
            || lower.starts_with("file://")
        {
            out.push(trimmed.to_owned());
        }
    }
    out
}

#[must_use]
pub(crate) fn extract_filenames(text: &str) -> Vec<String> {
    let clean = strip_citations(text);
    let mut out = Vec::new();
    for segment in clean.split(|c: char| !is_path_char(c)) {
        let trimmed = segment.trim_matches('.');
        if is_filename_token(trimmed) {
            let base = trimmed
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or(trimmed)
                .to_lowercase();
            if !out.contains(&base) {
                out.push(base);
            }
        }
    }
    out
}

fn is_path_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-' | '/' | '\\' | '@')
}

fn is_filename_token(token: &str) -> bool {
    if token.is_empty() || token.contains('@') {
        return false;
    }
    let Some((stem, ext)) = token.rsplit_once('.') else {
        return false;
    };
    !stem.is_empty()
        && ext.len() <= 6
        && ext.chars().all(|c| c.is_ascii_alphabetic())
        && !ext.is_empty()
}
