use std::path::Path;

use crate::collect::read_file_lossy;

pub const MAKEFILE_REL: &str = "Makefile";

#[derive(PartialEq, Eq)]
enum QuoteState {
    None,
    Single,
    Double,
}

fn scan_top_level(line: &str, visit: &mut impl FnMut(char, bool)) {
    let mut state = QuoteState::None;
    let mut escaped = false;
    for c in line.chars() {
        if escaped {
            escaped = false;
            visit(c, false);
            continue;
        }
        if c == '\\' && state != QuoteState::Single {
            escaped = true;
            visit(c, false);
            continue;
        }
        let quoted = state != QuoteState::None;
        if state == QuoteState::Single && c == '\'' {
            state = QuoteState::None;
            visit(c, false);
            continue;
        }
        if state == QuoteState::Double && c == '"' {
            state = QuoteState::None;
            visit(c, false);
            continue;
        }
        if !quoted && c == '\'' {
            state = QuoteState::Single;
            visit(c, false);
            continue;
        }
        if !quoted && c == '"' {
            state = QuoteState::Double;
            visit(c, false);
            continue;
        }
        visit(c, quoted);
    }
}

#[must_use]
pub fn split_top_level_semicolons(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut seg = String::new();
    scan_top_level(line, &mut |c, quoted| {
        if !quoted && c == ';' {
            out.push(std::mem::take(&mut seg));
        } else {
            seg.push(c);
        }
    });
    out.push(seg);
    out
}

#[must_use]
pub fn has_top_level_pipe(text: &str) -> bool {
    let mut found = false;
    scan_top_level(text, &mut |c, quoted| {
        if !quoted && c == '|' {
            found = true;
        }
    });
    found
}

#[must_use]
pub fn tokens(segment: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut word = String::new();
    scan_top_level(segment, &mut |c, quoted| {
        if !quoted && (c == ' ' || c == '\t') {
            if !word.is_empty() {
                out.push(std::mem::take(&mut word));
            }
        } else {
            word.push(c);
        }
    });
    if !word.is_empty() {
        out.push(word);
    }
    out
}

#[must_use]
pub fn paren_end(text: &str) -> Option<usize> {
    let mut depth = 1usize;
    let mut state = QuoteState::None;
    let mut escaped = false;
    for (idx, c) in text.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if c == '\\' && state != QuoteState::Single {
            escaped = true;
            continue;
        }
        if state == QuoteState::Single {
            if c == '\'' {
                state = QuoteState::None;
            }
            continue;
        }
        if state == QuoteState::Double {
            if c == '"' {
                state = QuoteState::None;
            }
            continue;
        }
        if c == '\'' {
            state = QuoteState::Single;
            continue;
        }
        if c == '"' {
            state = QuoteState::Double;
            continue;
        }
        if c == '(' {
            depth = depth.saturating_add(1);
        }
        if c == ')' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(idx);
            }
        }
    }
    None
}

#[must_use]
pub fn logical_recipe_lines(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut open: Option<(usize, String)> = None;
    for (idx, raw) in text.lines().enumerate() {
        let line_no = idx.saturating_add(1);
        if !raw.starts_with('\t') {
            if let Some(pair) = open.take() {
                out.push(pair);
            }
            continue;
        }
        let body = raw.trim_start_matches(['\t', '@', '-', '+']).to_owned();
        match open.as_mut() {
            Some((_, acc)) if acc.ends_with('\\') => {
                acc.pop();
                acc.push_str(&body);
            }
            _ => {
                if let Some(pair) = open.take() {
                    out.push(pair);
                }
                open = Some((line_no, body));
            }
        }
    }
    if let Some(pair) = open.take() {
        out.push(pair);
    }
    out
}

pub fn read_makefile(repo: &Path) -> Result<String, String> {
    read_file_lossy(&repo.join(MAKEFILE_REL))
}
