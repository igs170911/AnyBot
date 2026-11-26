use crate::sandbox_deny::normalize_command;
use crate::sandbox_deny::split_segments;
use crate::sandbox_subshell::SHELL_HEADS;
use crate::sandbox_subshell::take_subshells;

pub const PEEL_WRAPPERS: [&str; 11] = [
    "sudo", "doas", "env", "nohup", "nice", "time", "stdbuf", "timeout", "xargs", "command", "exec",
];

pub const PRIVILEGE_WRAPPERS: [&str; 2] = ["sudo", "doas"];

const COMPOUND_TOKENS: [&str; 10] = [
    "if", "then", "else", "fi", "while", "until", "do", "done", "case", "esac",
];

pub const CODE_TOOL_EVAL: &str = "tool-unreadable-eval";
pub const CODE_TOOL_UNBALANCED: &str = "tool-unreadable-quotes";
pub const CODE_TOOL_HEREDOC: &str = "tool-unreadable-heredoc";
pub const CODE_TOOL_HEAD: &str = "tool-unreadable-head";

#[must_use]
pub fn visible_segments(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    for segment in split_segments(line) {
        push_compound_splits(&segment, &mut out);
    }
    out
}

fn push_compound_splits(segment: &str, out: &mut Vec<String>) {
    let parts: Vec<&str> = segment.split_whitespace().collect();
    let mut start = 0usize;
    for (index, token) in parts.iter().enumerate() {
        if COMPOUND_TOKENS.contains(&token.to_ascii_lowercase().as_str()) {
            push_piece(parts.get(start..index).unwrap_or(&[]), out);
            start = index.saturating_add(1);
        }
    }
    push_piece(parts.get(start..).unwrap_or(&[]), out);
}

fn push_piece(parts: &[&str], out: &mut Vec<String>) {
    if parts.is_empty() {
        return;
    }
    let piece = normalize_command(&parts.join(" "));
    let (clean, inner) = take_subshells(&piece);
    if !clean.is_empty() {
        if let Some(script) = shell_script(&clean) {
            push_script_piece(&script, out);
        } else {
            out.push(clean);
        }
    }
    for text in inner {
        push_script_piece(&text, out);
    }
}

fn shell_script(segment: &str) -> Option<String> {
    let mut parts = segment.split_whitespace();
    let head = parts.next()?;
    let rest: Vec<&str> = parts.collect();
    if !SHELL_HEADS.contains(&bare_name(head).as_str()) || !rest.contains(&"-c") {
        return None;
    }
    script_argument(&rest)
}

fn push_script_piece(text: &str, out: &mut Vec<String>) {
    for segment in split_segments(text) {
        push_compound_splits(&segment, out);
    }
}

#[must_use]
pub fn peel_segment(segment: &str, bot_machine: bool) -> String {
    let mut current = segment.to_owned();
    for _ in 0..12 {
        let Some(next) = peel_head(&current, bot_machine) else {
            return current;
        };
        current = next;
    }
    current
}

fn peel_head(segment: &str, bot_machine: bool) -> Option<String> {
    let mut parts = segment.split_whitespace();
    let head = parts.next()?;
    let rest: Vec<&str> = parts.collect();
    if head.contains('=') {
        return non_empty_join(&rest);
    }
    let name = bare_name(head);
    if SHELL_HEADS.contains(&name.as_str()) && rest.contains(&"-c") {
        return script_argument(&rest);
    }
    if name != head.to_ascii_lowercase() {
        let rebuilt = format!("{name} {}", rest.join(" "));
        return normalize_command(&rebuilt).into_option();
    }
    if !PEEL_WRAPPERS.contains(&name.as_str()) {
        return None;
    }
    if PRIVILEGE_WRAPPERS.contains(&name.as_str()) && !bot_machine {
        return None;
    }
    peel_wrapper_args(name.as_str(), &rest)
}

fn peel_wrapper_args(name: &str, rest: &[&str]) -> Option<String> {
    let numeric = ["nice", "stdbuf", "timeout", "time"].contains(&name);
    let mut inner: Vec<&str> = Vec::new();
    let mut skipping = true;
    let mut value_expected = false;
    for token in rest {
        if skipping {
            if value_expected {
                value_expected = false;
                continue;
            }
            if crate::sandbox_deny::WRAPPER_VALUE_OPTIONS.contains(token) && !token.contains('=') {
                value_expected = true;
                continue;
            }
            if token.contains('=') || token.starts_with('-') {
                continue;
            }
            if numeric && unsigned_number(token) {
                continue;
            }
            skipping = false;
        }
        inner.push(token);
    }
    non_empty_join(&inner)
}

fn unsigned_number(token: &str) -> bool {
    token
        .strip_suffix('s')
        .unwrap_or(token)
        .parse::<u64>()
        .is_ok()
}

fn non_empty_join(parts: &[&str]) -> Option<String> {
    if parts.is_empty() {
        return None;
    }
    Some(normalize_command(&parts.join(" ")))
}

fn script_argument(rest: &[&str]) -> Option<String> {
    let index = rest.iter().position(|t| *t == "-c")?;
    let tail = rest.get(index.saturating_add(1)..).unwrap_or(&[]);
    let stripped = unquote(&tail.join(" "));
    normalize_command(&stripped).into_option()
}

trait IntoOption {
    fn into_option(self) -> Option<String>;
}

impl IntoOption for String {
    fn into_option(self) -> Option<String> {
        if self.is_empty() { None } else { Some(self) }
    }
}

#[must_use]
pub fn bare_name(head: &str) -> String {
    let unquoted = unquote(head);
    let last = unquoted.rsplit('/').next().unwrap_or(&unquoted);
    last.to_ascii_lowercase()
}

fn unquote(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.len() >= 2 {
        let first = trimmed.chars().next().unwrap_or('\0');
        let last_char = trimmed.chars().last().unwrap_or('\0');
        if (first == '\'' && last_char == '\'') || (first == '"' && last_char == '"') {
            let inner: String = trimmed
                .chars()
                .skip(1)
                .take(trimmed.len().saturating_sub(2))
                .collect();
            return inner;
        }
    }
    trimmed.to_owned()
}

#[must_use]
pub fn unreadable_reason(line: &str) -> Option<&'static str> {
    if line.split_whitespace().any(|t| t == "eval") {
        return Some(CODE_TOOL_EVAL);
    }
    if has_heredoc_into_shell(line) {
        return Some(CODE_TOOL_HEREDOC);
    }
    if crate::sandbox_levels::has_unbalanced_shape(line) {
        return Some(CODE_TOOL_UNBALANCED);
    }
    if visible_segments(line)
        .iter()
        .any(|s| unreadable_head(&peel_segment(s, false)))
    {
        return Some(CODE_TOOL_HEAD);
    }
    None
}

fn unreadable_head(segment: &str) -> bool {
    segment
        .split_whitespace()
        .next()
        .is_some_and(|head| head.starts_with('$') || head.starts_with('*'))
}

fn has_heredoc_into_shell(line: &str) -> bool {
    line.contains("<<")
        && line
            .split_whitespace()
            .any(|t| SHELL_HEADS.contains(&bare_name(t).as_str()))
}
