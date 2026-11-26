pub const DENIED_COMMAND_KINDS: [&str; 10] = [
    "sudo",
    "su",
    "rm -rf /",
    "rm -rf ~",
    "mkfs",
    "dd",
    "shutdown",
    "reboot",
    "diskutil erase",
    "chmod -R 777 /",
];

const NUMERIC_WRAPPERS: [&str; 4] = ["time", "nice", "stdbuf", "timeout"];

const COMMAND_WRAPPERS: [&str; 9] = [
    "env", "command", "nohup", "time", "nice", "stdbuf", "timeout", "xargs", "exec",
];

pub const WRAPPER_VALUE_OPTIONS: [&str; 12] = [
    "-u", "-g", "-l", "-c", "-d", "-e", "-i", "-n", "-p", "--user", "--group", "--login",
];

const JOIN_TOKENS: [&str; 6] = [";", "&&", "||", "|", "&", "\n"];

#[must_use]
pub fn normalize_command(text: &str) -> String {
    let lowered = text.to_lowercase();
    let mut out = String::with_capacity(lowered.len());
    let mut space = false;
    for c in lowered.chars() {
        if c.is_whitespace() {
            space = true;
            continue;
        }
        if space && !out.is_empty() {
            out.push(' ');
        }
        space = false;
        out.push(c);
    }
    out
}

#[must_use]
pub fn split_segments(line: &str) -> Vec<String> {
    let bytes = line.as_bytes();
    let mut raw = Vec::new();
    let mut start = 0usize;
    let mut pos = 0usize;
    while pos < bytes.len() {
        let Some(len) = join_token_len_at(bytes, pos) else {
            pos = pos.saturating_add(1);
            continue;
        };
        raw.push(bytes.get(start..pos).unwrap_or(&[]).to_vec());
        pos = pos.saturating_add(len);
        start = pos;
    }
    raw.push(bytes.get(start..).unwrap_or(&[]).to_vec());
    raw.into_iter()
        .map(|chunk| normalize_command(&String::from_utf8_lossy(&chunk)))
        .filter(|s| !s.is_empty())
        .collect()
}

fn join_token_len_at(bytes: &[u8], pos: usize) -> Option<usize> {
    JOIN_TOKENS
        .iter()
        .filter(|t| starts_with_at(bytes, pos, t.as_bytes()))
        .map(|t| t.len())
        .max()
}

fn starts_with_at(bytes: &[u8], pos: usize, token: &[u8]) -> bool {
    let Some(window) = bytes.get(pos..pos.saturating_add(token.len())) else {
        return false;
    };
    window == token
}

#[must_use]
pub fn strip_wrappers(segment: &str) -> String {
    let mut current = segment.to_owned();
    for _ in 0..8 {
        let Some(inner) = peel_wrapper(&current) else {
            return current;
        };
        current = inner;
    }
    current
}

fn peel_wrapper(segment: &str) -> Option<String> {
    let mut parts = segment.split_whitespace();
    let head = parts.next()?;
    if head.contains('=') {
        let kept: Vec<&str> = parts.collect();
        if kept.is_empty() {
            return None;
        }
        return Some(normalize_command(&kept.join(" ")));
    }
    if !COMMAND_WRAPPERS.contains(&head) {
        return None;
    }
    let numeric = NUMERIC_WRAPPERS.contains(&head);
    let mut inner: Vec<&str> = Vec::new();
    let mut skipping = true;
    let mut value_expected = false;
    for token in parts {
        if skipping {
            if value_expected {
                value_expected = false;
                continue;
            }
            if WRAPPER_VALUE_OPTIONS.contains(&token) && !token.contains('=') {
                value_expected = true;
                continue;
            }
            if is_wrapper_arg(token, numeric) {
                continue;
            }
            skipping = false;
        }
        inner.push(token);
    }
    if inner.is_empty() {
        return None;
    }
    Some(normalize_command(&inner.join(" ")))
}

fn is_wrapper_arg(token: &str, numeric: bool) -> bool {
    if token.contains('=') || token.starts_with('-') {
        return true;
    }
    numeric && unsigned_number(token).is_some()
}

fn unsigned_number(token: &str) -> Option<u64> {
    let digits = token.strip_suffix('s').unwrap_or(token);
    digits.parse::<u64>().ok()
}

const PRIVILEGE_WRAPPERS: [&str; 2] = ["sudo", "doas"];

#[must_use]
pub fn deny_token(line: &str, bot_machine: bool) -> Option<&'static str> {
    for segment in crate::sandbox_peel::visible_segments(line) {
        let inner = crate::sandbox_peel::peel_segment(&segment, bot_machine);
        for kind in DENIED_COMMAND_KINDS {
            if inner.starts_with(normalize_command(kind).as_str()) {
                return Some(kind);
            }
        }
    }
    None
}

#[must_use]
pub fn strip_privilege_wrappers(segment: &str) -> String {
    let mut current = segment.to_owned();
    for _ in 0..8 {
        let Some(inner) = peel_privilege(&current) else {
            return current;
        };
        current = inner;
    }
    current
}

const PRIVILEGE_VALUE_OPTIONS: [&str; 8] = [
    "-u",
    "-g",
    "-l",
    "-c",
    "--user",
    "--group",
    "--login",
    "--command",
];

fn peel_privilege(segment: &str) -> Option<String> {
    let mut parts = segment.split_whitespace();
    let head = parts.next()?;
    if !PRIVILEGE_WRAPPERS.contains(&head) {
        return None;
    }
    let mut inner: Vec<&str> = Vec::new();
    let mut skipping = true;
    let mut value_expected = false;
    for token in parts {
        if skipping {
            if value_expected {
                value_expected = false;
                continue;
            }
            if PRIVILEGE_VALUE_OPTIONS.contains(&token) && !token.contains('=') {
                value_expected = true;
                continue;
            }
            if token.starts_with('-') {
                continue;
            }
            skipping = false;
        }
        inner.push(token);
    }
    if inner.is_empty() {
        return None;
    }
    Some(normalize_command(&inner.join(" ")))
}
