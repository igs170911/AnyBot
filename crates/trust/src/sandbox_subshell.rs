use crate::sandbox_deny::normalize_command;

pub const SHELL_HEADS: [&str; 6] = ["sh", "bash", "zsh", "ksh", "dash", "fish"];

pub(crate) fn take_subshells(text: &str) -> (String, Vec<String>) {
    let mut inner = Vec::new();
    let mut out = String::new();
    let mut pending = String::new();
    let mut depth = 0usize;
    let mut mode = ScanMode::Plain;
    for c in text.chars() {
        if mode == ScanMode::Plain {
            if c == '`' {
                mode = ScanMode::Tick;
                flush(&mut pending, &mut out);
                continue;
            }
            if c == '(' && pending.ends_with('$') {
                pending.pop();
                flush(&mut pending, &mut out);
                mode = ScanMode::Paren;
                depth = 1;
                continue;
            }
            if c == '(' && is_subshell_start(&out, &pending) {
                trim_out(&mut out, &mut pending);
                mode = ScanMode::Paren;
                depth = 1;
                continue;
            }
            pending.push(c);
            continue;
        }
        collect_scanned(c, &mut mode, &mut depth, &mut pending, &mut inner);
    }
    finish_scan(&mut mode, &mut pending, &mut inner);
    flush(&mut pending, &mut out);
    (out, inner)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScanMode {
    Plain,
    Tick,
    Paren,
}

fn collect_scanned(
    c: char,
    mode: &mut ScanMode,
    depth: &mut usize,
    pending: &mut String,
    inner: &mut Vec<String>,
) {
    if *mode == ScanMode::Tick {
        if c == '`' {
            close_scan(pending, inner);
            *mode = ScanMode::Plain;
            return;
        }
        pending.push(c);
        return;
    }
    if c == '(' {
        *depth = depth.saturating_add(1);
    }
    if c == ')' {
        *depth = depth.saturating_sub(1);
        if *depth == 0 {
            close_scan(pending, inner);
            *mode = ScanMode::Plain;
            return;
        }
    }
    pending.push(c);
}

fn finish_scan(mode: &mut ScanMode, pending: &mut String, inner: &mut Vec<String>) {
    if *mode != ScanMode::Plain {
        close_scan(pending, inner);
        *mode = ScanMode::Plain;
    }
}

fn close_scan(pending: &mut String, inner: &mut Vec<String>) {
    let cleaned = normalize_command(pending);
    pending.clear();
    if !cleaned.is_empty() {
        inner.push(cleaned);
    }
}

pub(crate) fn is_subshell_start(out: &str, pending: &str) -> bool {
    let joined = format!("{out}{pending}");
    joined
        .split_whitespace()
        .last()
        .is_some_and(|t| SHELL_HEADS.contains(&t) || t == "echo" || t == "cat")
}

fn trim_out(out: &mut String, pending: &mut String) {
    pending.clear();
    let trimmed = out.trim_end();
    let cut = trimmed.len();
    out.truncate(cut);
}

fn flush(pending: &mut String, out: &mut String) {
    out.push_str(pending);
    pending.clear();
}
