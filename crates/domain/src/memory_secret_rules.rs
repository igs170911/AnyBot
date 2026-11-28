#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretMatch {
    pub pattern: &'static str,
    pub start: usize,
    pub end: usize,
}

const PREFIX_RULES: [(&str, &str, usize); 17] = [
    ("anthropic-key", "sk-ant-", 8),
    ("openai-proj-key", "sk-proj-", 8),
    ("openai-svc-key", "sk-svcacct-", 8),
    ("stripe-live-key", "sk_live_", 8),
    ("stripe-rk-key", "rk_live_", 8),
    ("openai-sk-key", "sk-", 10),
    ("github-pat", "ghp_", 8),
    ("github-oauth", "gho_", 8),
    ("github-user", "ghu_", 8),
    ("github-server", "ghs_", 8),
    ("github-refresh", "ghr_", 8),
    ("github-fine-pat", "github_pat_", 8),
    ("slack-bot-token", "xoxb-", 8),
    ("slack-user-token", "xoxp-", 8),
    ("slack-app-token", "xoxa-", 8),
    ("slack-refresh-token", "xoxr-", 8),
    ("google-api-key", "AIza", 16),
];

const KV_KEYS: [&str; 8] = [
    "password",
    "passwd",
    "secret",
    "api_key",
    "apikey",
    "access_token",
    "auth_token",
    "private_key",
];

#[must_use]
pub fn find_secret_patterns(text: &str) -> Vec<SecretMatch> {
    let mut found = Vec::new();
    collect_prefix_matches(text, &mut found);
    collect_aws_matches(text, &mut found);
    collect_pem_matches(text, &mut found);
    collect_bearer_matches(text, &mut found);
    collect_kv_matches(text, &mut found);
    found.sort_by_key(|item| (item.start, item.end));
    dedup_overlapping(found)
}

#[must_use]
pub fn has_secret_pattern(text: &str) -> bool {
    !find_secret_patterns(text).is_empty()
}

#[must_use]
pub fn redact_secret_patterns(text: &str) -> String {
    let matches = find_secret_patterns(text);
    if matches.is_empty() {
        return text.to_owned();
    }
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0usize;
    for item in matches {
        if item.start > cursor
            && let Some(segment) = text.get(cursor..item.start)
        {
            out.push_str(segment);
        }
        out.push_str("[redacted]");
        cursor = cursor.max(item.end);
    }
    if let Some(tail) = text.get(cursor..) {
        out.push_str(tail);
    }
    out
}

fn collect_prefix_matches(text: &str, out: &mut Vec<SecretMatch>) {
    for &(label, prefix, min_tail) in &PREFIX_RULES {
        for (start, _) in text.match_indices(prefix) {
            if !boundary_before(text, start) {
                continue;
            }
            let after_start = start.saturating_add(prefix.len());
            let Some(rest) = text.get(after_start..) else {
                continue;
            };
            let tail_len = scan_token_len(rest, is_key_token_char);
            if tail_len >= min_tail {
                out.push(SecretMatch {
                    pattern: label,
                    start,
                    end: after_start.saturating_add(tail_len),
                });
            }
        }
    }
}

fn collect_aws_matches(text: &str, out: &mut Vec<SecretMatch>) {
    for prefix in ["AKIA", "ASIA"] {
        for (start, _) in text.match_indices(prefix) {
            if !boundary_before(text, start) {
                continue;
            }
            let after_start = start.saturating_add(prefix.len());
            let Some(rest) = text.get(after_start..) else {
                continue;
            };
            let tail_len = scan_token_len(rest, |b| b.is_ascii_uppercase() || b.is_ascii_digit());
            if tail_len == 16 && boundary_after(rest, tail_len) {
                out.push(SecretMatch {
                    pattern: "aws-access-key",
                    start,
                    end: after_start.saturating_add(tail_len),
                });
            }
        }
    }
}

fn collect_pem_matches(text: &str, out: &mut Vec<SecretMatch>) {
    let header = "-----BEGIN ";
    let marker = "PRIVATE KEY-----";
    for (start, _) in text.match_indices(header) {
        let Some(rest) = text.get(start..) else {
            continue;
        };
        if let Some(marker_rel) = rest.find(marker) {
            let end = start
                .saturating_add(marker_rel)
                .saturating_add(marker.len());
            out.push(SecretMatch {
                pattern: "pem-private-key",
                start,
                end,
            });
        }
    }
}

fn collect_bearer_matches(text: &str, out: &mut Vec<SecretMatch>) {
    let prefix = "bearer ";
    for (start, _) in text.char_indices() {
        if !boundary_before(text, start) {
            continue;
        }
        let Some(rest) = text.get(start..) else {
            continue;
        };
        if !starts_with_ascii_ci(rest, prefix) {
            continue;
        }
        let Some(after) = rest.get(prefix.len()..) else {
            continue;
        };
        let ws = after.bytes().take_while(|b| *b == b' ').count();
        let Some(tok) = after.get(ws..) else {
            continue;
        };
        let tok_len = scan_token_len(tok, is_bearer_token_char);
        if tok_len >= 8 {
            let end = start
                .saturating_add(prefix.len())
                .saturating_add(ws)
                .saturating_add(tok_len);
            out.push(SecretMatch {
                pattern: "bearer-token",
                start,
                end,
            });
        }
    }
}

fn collect_kv_matches(text: &str, out: &mut Vec<SecretMatch>) {
    for (start, _) in text.char_indices() {
        if !boundary_before(text, start) {
            continue;
        }
        let Some(rest) = text.get(start..) else {
            continue;
        };
        for key in KV_KEYS {
            if !starts_with_ascii_ci(rest, key) {
                continue;
            }
            if let Some(end) = match_kv_value(rest, key.len(), start) {
                out.push(SecretMatch {
                    pattern: "credential-assignment",
                    start,
                    end,
                });
            }
        }
    }
}

fn match_kv_value(rest: &str, key_len: usize, base: usize) -> Option<usize> {
    let after_key = rest.get(key_len..)?;
    let spaces_before = after_key.bytes().take_while(|b| *b == b' ').count();
    let after_sp = after_key.get(spaces_before..)?;
    let after_eq = after_sp.strip_prefix('=')?;
    let spaces_after = after_eq.bytes().take_while(|b| *b == b' ').count();
    let val_slice = after_eq.get(spaces_after..)?;
    let val_len = scan_token_len(val_slice, is_kv_val_char);
    let raw_val = val_slice.get(..val_len)?;
    let trimmed = raw_val.trim_matches(|c| matches!(c, '"' | '\'' | ',' | ';'));
    if trimmed.len() < 4 || is_placeholder_value(trimmed) {
        return None;
    }
    Some(
        base.saturating_add(key_len)
            .saturating_add(spaces_before)
            .saturating_add(1)
            .saturating_add(spaces_after)
            .saturating_add(val_len),
    )
}

fn is_placeholder_value(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "null" | "none" | "empty" | "redacted" | "[redacted]" | "<redacted>" | "true" | "false"
    )
}

fn starts_with_ascii_ci(text: &str, prefix: &str) -> bool {
    text.as_bytes()
        .get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix.as_bytes()))
}

fn boundary_before(text: &str, start: usize) -> bool {
    text.get(..start)
        .and_then(|head| head.chars().next_back())
        .is_none_or(|ch| !ch.is_ascii_alphanumeric() && ch != '_')
}

fn boundary_after(rest: &str, offset: usize) -> bool {
    rest.get(offset..)
        .and_then(|tail| tail.chars().next())
        .is_none_or(|ch| !ch.is_ascii_alphanumeric() && ch != '_')
}

fn scan_token_len(text: &str, pred: fn(u8) -> bool) -> usize {
    text.bytes().take_while(|b| pred(*b)).count()
}

fn is_key_token_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.')
}

fn is_bearer_token_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'=' | b'+' | b'/')
}

fn is_kv_val_char(byte: u8) -> bool {
    !byte.is_ascii_whitespace() && byte != b'\n' && byte != b'\r'
}

fn dedup_overlapping(items: Vec<SecretMatch>) -> Vec<SecretMatch> {
    let mut kept: Vec<SecretMatch> = Vec::new();
    for item in items {
        if let Some(last) = kept.last_mut()
            && item.start < last.end
        {
            if item.end > last.end {
                last.end = item.end;
            }
            continue;
        }
        kept.push(item);
    }
    kept
}
