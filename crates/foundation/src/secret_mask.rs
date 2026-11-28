const MASK_DOTS: &str = "\u{2022}\u{2022}\u{2022}\u{2022}";

const TOKEN_PREFIXES: [&str; 9] = [
    "github_pat_",
    "glpat-",
    "ghp_",
    "xai-",
    "AKIA",
    "ASIA",
    "AIza",
    "sk-",
    "xox",
];

#[must_use]
pub fn mask_secrets(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut idx = 0usize;
    let mut prev: Option<char> = None;
    while idx < text.len() {
        let Some(rest) = text.get(idx..) else {
            break;
        };
        if let Some((masked, consumed)) = try_mask(rest, prev) {
            out.push_str(&masked);
            prev = masked.chars().last();
            idx = idx.saturating_add(consumed);
            continue;
        }
        let Some(ch) = rest.chars().next() else {
            break;
        };
        out.push(ch);
        prev = Some(ch);
        idx = idx.saturating_add(ch.len_utf8());
    }
    out
}

fn try_mask(rest: &str, prev: Option<char>) -> Option<(String, usize)> {
    if let Some(hit) = try_pem_private_key(rest) {
        return Some(hit);
    }
    if !is_boundary(prev) {
        return None;
    }
    if let Some(hit) = try_auth_scheme(rest) {
        return Some(hit);
    }
    if let Some(hit) = try_named_assignment(rest) {
        return Some(hit);
    }
    if let Some(hit) = try_jwt(rest) {
        return Some(hit);
    }
    try_prefix_token(rest)
}

fn is_boundary(prev: Option<char>) -> bool {
    match prev {
        None => true,
        Some(ch) => !(ch.is_ascii_alphanumeric() || ch == '_'),
    }
}

fn try_pem_private_key(rest: &str) -> Option<(String, usize)> {
    if !rest.starts_with("-----BEGIN ") {
        return None;
    }
    let header_end = rest.get(11..)?.find("-----")?;
    let header_len = 11usize.saturating_add(header_end).saturating_add(5);
    let header = rest.get(..header_len)?;
    if !header.contains("PRIVATE KEY") {
        return None;
    }
    let after = rest.get(header_len..)?;
    if let Some(end_start) = after.find("-----END ") {
        let footer_rest = after.get(end_start.saturating_add(9)..)?;
        if let Some(footer_end) = footer_rest.find("-----") {
            let total = header_len
                .saturating_add(end_start)
                .saturating_add(9)
                .saturating_add(footer_end)
                .saturating_add(5);
            let footer_start = header_len.saturating_add(end_start);
            let footer = rest.get(footer_start..total)?;
            return Some((format!("{header}\n{MASK_DOTS}\n{footer}"), total));
        }
    }
    Some((format!("{header}\n{MASK_DOTS}"), rest.len()))
}

fn try_auth_scheme(rest: &str) -> Option<(String, usize)> {
    let prefix_len = if starts_ignore_ascii_case(rest, "bearer") {
        6
    } else if starts_ignore_ascii_case(rest, "basic") {
        5
    } else {
        return None;
    };
    let after_word = rest.get(prefix_len..)?;
    let spaces = count_while(after_word, |c| c == ' ' || c == '\t');
    if spaces == 0 {
        return None;
    }
    let head_len = prefix_len.saturating_add(spaces);
    let head = rest.get(..head_len)?;
    let val_rest = rest.get(head_len..)?;
    let tok_len = count_while(val_rest, is_auth_token_char);
    if tok_len == 0 {
        return None;
    }
    let tok = val_rest.get(..tok_len)?;
    let masked = mask_value(tok);
    Some((format!("{head}{masked}"), head_len.saturating_add(tok_len)))
}

fn try_named_assignment(rest: &str) -> Option<(String, usize)> {
    let (quote_open, name_start) = match rest.chars().next() {
        Some(q @ ('"' | '\'')) => (Some(q), 1usize),
        _ => (None, 0usize),
    };
    let name_slice = rest.get(name_start..)?;
    let name_len = count_while(name_slice, |c| {
        c.is_ascii_alphanumeric() || c == '_' || c == '-'
    });
    if name_len == 0 {
        return None;
    }
    let name = name_slice.get(..name_len)?;
    if !is_secret_var_name(name) {
        return None;
    }
    let mut pos = name_start.saturating_add(name_len);
    if let Some(q) = quote_open {
        if rest.get(pos..)?.chars().next()? != q {
            return None;
        }
        pos = pos.saturating_add(1);
    }
    pos = pos.saturating_add(count_while(rest.get(pos..)?, |c| c == ' ' || c == '\t'));
    let sep = rest.get(pos..)?.chars().next()?;
    if sep != '=' && sep != ':' {
        return None;
    }
    pos = pos.saturating_add(1);
    pos = pos.saturating_add(count_while(rest.get(pos..)?, |c| c == ' ' || c == '\t'));
    let head = rest.get(..pos)?;
    let after_sep = rest.get(pos..)?;
    mask_assigned_value(head, pos, after_sep)
}

fn mask_assigned_value(head: &str, pos: usize, after_sep: &str) -> Option<(String, usize)> {
    if let Some(q @ ('"' | '\'')) = after_sep.chars().next() {
        let inner = after_sep.get(1..)?;
        let close_offset = inner.find(q)?;
        let content = inner.get(..close_offset)?;
        if content.contains('\n') || content.is_empty() {
            return None;
        }
        let masked = mask_value(content);
        let total = pos.saturating_add(2).saturating_add(close_offset);
        return Some((format!("{head}{q}{masked}{q}"), total));
    }
    let val_len = count_while(after_sep, is_unquoted_val_char);
    if val_len == 0 {
        return None;
    }
    let content = after_sep.get(..val_len)?;
    let masked = mask_value(content);
    Some((format!("{head}{masked}"), pos.saturating_add(val_len)))
}

fn try_jwt(rest: &str) -> Option<(String, usize)> {
    if !rest.starts_with("eyJ") {
        return None;
    }
    let s1 = count_while(rest, is_jwt_char);
    if s1 <= 3 {
        return None;
    }
    let after1 = rest.get(s1..)?;
    if !after1.starts_with('.') {
        return None;
    }
    let s2 = count_while(after1.get(1..)?, is_jwt_char);
    if s2 == 0 {
        return None;
    }
    let dot2_pos = s1.saturating_add(1).saturating_add(s2);
    let after2 = rest.get(dot2_pos..)?;
    if !after2.starts_with('.') {
        return None;
    }
    let s3 = count_while(after2.get(1..)?, is_jwt_char);
    let total = dot2_pos.saturating_add(1).saturating_add(s3);
    if total < 9 {
        return None;
    }
    Some((format!("eyJ{MASK_DOTS}"), total))
}

fn try_prefix_token(rest: &str) -> Option<(String, usize)> {
    for prefix in TOKEN_PREFIXES {
        if let Some(tail) = rest.strip_prefix(prefix) {
            let tail_len = count_while(tail, is_token_body_char);
            let min_len = if prefix == "xox" { 4 } else { 3 };
            if tail_len >= min_len {
                let total = prefix.len().saturating_add(tail_len);
                return Some((format!("{prefix}{MASK_DOTS}"), total));
            }
        }
    }
    None
}

fn mask_value(value: &str) -> String {
    if let Some((masked, len)) = try_jwt(value)
        && len == value.len()
    {
        return masked;
    }
    for prefix in TOKEN_PREFIXES {
        if value.starts_with(prefix) && value.len() > prefix.len() {
            return format!("{prefix}{MASK_DOTS}");
        }
    }
    MASK_DOTS.to_owned()
}

fn is_secret_var_name(name: &str) -> bool {
    let norm: String = name
        .chars()
        .map(|c| {
            if c == '-' {
                '_'
            } else {
                c.to_ascii_uppercase()
            }
        })
        .collect();
    norm.contains("TOKEN")
        || norm.contains("SECRET")
        || norm.contains("PASSWORD")
        || norm.contains("PASSWD")
        || norm.contains("API_KEY")
        || norm.contains("APIKEY")
}

fn starts_ignore_ascii_case(text: &str, prefix: &str) -> bool {
    text.get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
}

fn count_while(text: &str, pred: fn(char) -> bool) -> usize {
    let mut len = 0usize;
    for ch in text.chars() {
        if !pred(ch) {
            break;
        }
        len = len.saturating_add(ch.len_utf8());
    }
    len
}

fn is_auth_token_char(ch: char) -> bool {
    !ch.is_ascii_whitespace()
        && !matches!(
            ch,
            '"' | '\'' | '`' | ',' | ';' | ')' | ']' | '}' | '<' | '>'
        )
}

fn is_unquoted_val_char(ch: char) -> bool {
    !ch.is_ascii_whitespace() && !matches!(ch, '"' | '\'' | ',' | ';' | '&' | ')' | ']' | '}')
}

fn is_token_body_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'
}

fn is_jwt_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'
}
