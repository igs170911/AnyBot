use crate::pattern_glob::{Levels, expand_home, glob_match, normalize_path};
use crate::pattern_web::domain_covers;
use crate::sandbox_peel::visible_segments;

const DOMAIN_PREFIX: &str = "domain:";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RulePattern {
    Shell(String),
    File(String),
    Connector(String),
    Web { domain: String },
    Tool(String),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PatternError {
    #[error("pattern is empty")]
    Empty,
    #[error("parentheses are not balanced")]
    Unbalanced,
    #[error("unknown pattern kind: {0}")]
    UnknownKind(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebVia {
    Opened,
    Redirect,
    LinkClick,
    Back,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternTarget {
    Command { line: String },
    Path { path: String },
    Connector { server: String, tool: String },
    Web { url: String, via: WebVia },
    Step { name: String },
}

pub fn parse_pattern(text: &str) -> Result<RulePattern, PatternError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(PatternError::Empty);
    }
    let (kind, inner) = split_kind(trimmed)?;
    if inner.is_empty() {
        return Err(PatternError::Empty);
    }
    match kind {
        "shell" => Ok(RulePattern::Shell(inner.to_owned())),
        "file" => Ok(RulePattern::File(inner.to_owned())),
        "connector" => Ok(RulePattern::Connector(inner.to_owned())),
        "web" => Ok(RulePattern::Web {
            domain: inner
                .strip_prefix(DOMAIN_PREFIX)
                .unwrap_or_default()
                .trim()
                .to_owned(),
        }),
        "tool" => Ok(RulePattern::Tool(inner.to_owned())),
        other => Err(PatternError::UnknownKind(other.to_owned())),
    }
}

fn split_kind(text: &str) -> Result<(&str, &str), PatternError> {
    let open = text.find('(').ok_or(PatternError::Unbalanced)?;
    if !text.ends_with(')') || !parens_balanced(text) {
        return Err(PatternError::Unbalanced);
    }
    let kind = text.get(..open).unwrap_or_default().trim();
    let inner = text
        .get(open.saturating_add(1)..text.len().saturating_sub(1))
        .unwrap_or_default()
        .trim();
    Ok((kind, inner))
}

fn parens_balanced(text: &str) -> bool {
    let mut depth: u32 = 0;
    for ch in text.chars() {
        if ch == '(' {
            depth = depth.saturating_add(1);
        } else if ch == ')' {
            let Some(rest) = depth.checked_sub(1) else {
                return false;
            };
            depth = rest;
        }
    }
    depth == 0
}

#[must_use]
pub fn pattern_text(pattern: &RulePattern) -> String {
    match pattern {
        RulePattern::Shell(glob) => format!("shell({glob})"),
        RulePattern::File(glob) => format!("file({glob})"),
        RulePattern::Connector(glob) => format!("connector({glob})"),
        RulePattern::Web { domain } => format!("web({DOMAIN_PREFIX}{domain})"),
        RulePattern::Tool(glob) => format!("tool({glob})"),
    }
}

#[must_use]
pub fn matches_nothing(pattern: &RulePattern) -> bool {
    matches!(pattern, RulePattern::Web { domain } if domain.is_empty())
}

#[must_use]
pub fn allow_hits(pattern: &RulePattern, target: &PatternTarget, home: &str) -> bool {
    if let (RulePattern::Shell(glob), PatternTarget::Command { line }) = (pattern, target) {
        return shell_allows(glob, line);
    }
    other_hits(pattern, target, home)
}

#[must_use]
pub fn ask_hits(pattern: &RulePattern, target: &PatternTarget, home: &str) -> bool {
    if let (RulePattern::Shell(glob), PatternTarget::Command { line }) = (pattern, target) {
        return shell_asks(glob, line);
    }
    other_hits(pattern, target, home)
}

fn other_hits(pattern: &RulePattern, target: &PatternTarget, home: &str) -> bool {
    match (pattern, target) {
        (RulePattern::File(glob), PatternTarget::Path { path }) => file_hits(glob, path, home),
        (RulePattern::Connector(glob), PatternTarget::Connector { server, tool }) => {
            glob_match(glob, &format!("{server}__{tool}"), Levels::Flat)
        }
        (RulePattern::Web { domain }, PatternTarget::Web { url, via }) => {
            *via == WebVia::Opened && !domain.is_empty() && domain_covers(domain, url)
        }
        (RulePattern::Tool(glob), PatternTarget::Step { name }) => {
            glob_match(glob, name, Levels::Flat)
        }
        (
            RulePattern::Shell(_)
            | RulePattern::File(_)
            | RulePattern::Connector(_)
            | RulePattern::Web { .. }
            | RulePattern::Tool(_),
            _,
        ) => false,
    }
}

fn has_substitution(line: &str) -> bool {
    line.contains("$(") || line.contains('`')
}

fn shell_allows(glob: &str, line: &str) -> bool {
    if has_substitution(line) {
        return false;
    }
    let segments = visible_segments(line);
    !segments.is_empty()
        && segments
            .iter()
            .all(|segment| glob_match(glob, segment, Levels::Flat))
}

fn shell_asks(glob: &str, line: &str) -> bool {
    let segments = visible_segments(line);
    if segments.is_empty() {
        return glob_match(glob, line.trim(), Levels::Flat);
    }
    segments
        .iter()
        .any(|segment| glob_match(glob, segment, Levels::Flat))
}

fn file_hits(glob: &str, path: &str, home: &str) -> bool {
    let Some(target) = normalize_path(path, home) else {
        return false;
    };
    glob_match(&expand_home(glob, home), &target, Levels::Slashes)
}
