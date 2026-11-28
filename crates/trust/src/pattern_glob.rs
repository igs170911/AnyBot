#[derive(Clone, Copy, PartialEq, Eq)]
enum Token {
    Lit(char),
    One,
    Star,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Levels {
    Flat,
    Slashes,
}

fn tokens_of(pattern: &str, levels: Levels) -> Vec<(Token, bool)> {
    let mut out: Vec<(Token, bool)> = Vec::new();
    let mut chars = pattern.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '*' => {
                let mut deep = false;
                while chars.peek() == Some(&'*') {
                    chars.next();
                    deep = true;
                }
                out.push((Token::Star, deep || levels == Levels::Flat));
            }
            '?' => out.push((Token::One, levels == Levels::Flat)),
            other => out.push((Token::Lit(other), true)),
        }
    }
    out
}

pub(crate) fn glob_match(pattern: &str, text: &str, levels: Levels) -> bool {
    let tokens = tokens_of(pattern, levels);
    let chars: Vec<char> = text.chars().collect();
    let mut reach: Vec<bool> = vec![false; chars.len().saturating_add(1)];
    if let Some(first) = reach.first_mut() {
        *first = true;
    }
    for (token, crosses) in tokens {
        reach = step(&reach, &chars, token, crosses);
    }
    reach.last().copied().unwrap_or(false)
}

fn step(reach: &[bool], chars: &[char], token: Token, crosses: bool) -> Vec<bool> {
    let mut next: Vec<bool> = vec![false; reach.len()];
    for index in 0..reach.len() {
        let here = reach.get(index).copied().unwrap_or(false);
        let ch = chars.get(index).copied();
        match token {
            Token::Lit(want) => set(&mut next, index.saturating_add(1), here && ch == Some(want)),
            Token::One => set(
                &mut next,
                index.saturating_add(1),
                here && ch.is_some_and(|c| crosses || c != '/'),
            ),
            Token::Star => {
                let carried = index
                    .checked_sub(1)
                    .and_then(|prev| Some((next.get(prev).copied()?, chars.get(prev).copied()?)))
                    .is_some_and(|(open, c)| open && (crosses || c != '/'));
                set(&mut next, index, here || carried);
            }
        }
    }
    next
}

fn set(slots: &mut [bool], index: usize, value: bool) {
    if value && let Some(slot) = slots.get_mut(index) {
        *slot = true;
    }
}

pub(crate) fn normalize_path(path: &str, home: &str) -> Option<String> {
    let expanded = expand_home(path, home);
    let absolute = expanded.starts_with('/');
    let mut parts: Vec<&str> = Vec::new();
    for part in expanded.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }
    let joined = parts.join("/");
    if absolute {
        return Some(format!("/{joined}"));
    }
    Some(joined)
}

pub(crate) fn expand_home(path: &str, home: &str) -> String {
    if path == "~" {
        return home.to_owned();
    }
    if let Some(rest) = path.strip_prefix("~/") {
        return format!("{}/{rest}", home.trim_end_matches('/'));
    }
    path.to_owned()
}
