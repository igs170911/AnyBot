use std::path::Path;

use crate::makefile_scan::has_top_level_pipe;
use crate::makefile_scan::logical_recipe_lines;
use crate::makefile_scan::paren_end;
use crate::makefile_scan::read_makefile;
use crate::makefile_scan::split_top_level_semicolons;
use crate::makefile_scan::tokens;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum Segment {
    Structural,
    Condition,
    SafeAssignment,
    Failable,
    Terminal,
}

const LEADING_KEYWORDS: [&str; 9] = ["do", "then", "else", "elif", "fi", "done", "esac", "{", "}"];
const CONDITION_HEADS: [&str; 4] = ["if", "for", "while", "until"];
const TERMINAL_HEADS: [&str; 5] = ["exit", "return", "break", "continue", "set"];
const FAILABLE_SUBSTITUTIONS: [&str; 4] = ["$(", "<(", ">(", "`"];

fn strip_leading_keywords(segment: &str) -> Vec<String> {
    let mut rest = tokens(segment);
    while let Some(first) = rest.first() {
        if LEADING_KEYWORDS.contains(&first.as_str()) {
            rest.remove(0);
        } else {
            break;
        }
    }
    rest
}

fn classify(segment: &str) -> Segment {
    let rest = strip_leading_keywords(segment);
    let Some(head) = rest.first() else {
        return Segment::Structural;
    };
    let head = head.as_str();
    if head.starts_with('<') || head.starts_with('>') {
        return Segment::Structural;
    }
    if CONDITION_HEADS.contains(&head) {
        return Segment::Condition;
    }
    if TERMINAL_HEADS.contains(&head) {
        return Segment::Terminal;
    }
    if head == "!" {
        return Segment::Failable;
    }
    if is_safe_assignment(&rest) {
        return Segment::SafeAssignment;
    }
    Segment::Failable
}

fn is_safe_assignment(tokens: &[String]) -> bool {
    let Some((name, _value)) = tokens.first().and_then(|t| t.split_once('=')) else {
        return false;
    };
    if name.is_empty()
        || name.chars().next().is_some_and(|c| c.is_ascii_digit())
        || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return false;
    }
    let joined = tokens.join(" ");
    if FAILABLE_SUBSTITUTIONS.iter().any(|m| joined.contains(m)) {
        return false;
    }
    !tokens
        .get(1..)
        .into_iter()
        .flatten()
        .any(|t| !t.contains('=') && t.starts_with(|c: char| c.is_ascii_alphanumeric()))
}

fn swallow_message(line_no: usize, left: &str, right: &str) -> String {
    format!(
        "Makefile:{line_no}: `;` between \"{left}\" and \"{right}\" swallows the left command's exit status; chain with &&, guard with a condition, or call a script (spec/05 section 2)"
    )
}

fn semicolon_violations(line_no: usize, line: &str) -> Vec<String> {
    let mut out = Vec::new();
    if line.contains("set -e") {
        return out;
    }
    let segments = split_top_level_semicolons(line);
    for pair in segments.windows(2) {
        let [left, right] = pair else { continue };
        let left = left.trim();
        let right = right.trim();
        if classify(left) == Segment::Failable && classify(right) == Segment::Failable {
            out.push(swallow_message(line_no, left, right));
        }
    }
    out
}

fn shell_pipefail_violations(text: &str, recipes: &[(usize, String)]) -> Vec<String> {
    let needs_pipefail = recipes.iter().any(|(_, l)| has_top_level_pipe(l));
    let shell_set = text
        .lines()
        .any(|l| l.starts_with("SHELL") && l.contains("pipefail"));
    if needs_pipefail && !shell_set {
        return vec![
            "Makefile: SHELL must enable pipefail so pipeline failures block recipe lines (spec/05 section 2)"
                .to_owned(),
        ];
    }
    Vec::new()
}

fn make_shell_calls(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        let line_no = idx.saturating_add(1);
        let mut cursor = 0usize;
        while let Some(tail) = line.get(cursor..) {
            let Some(at) = tail.find("$(shell") else {
                break;
            };
            let head_len = "$(shell".len();
            let start = cursor.saturating_add(at).saturating_add(head_len);
            let Some(rest) = line.get(start..) else { break };
            let Some(close) = paren_end(rest) else { break };
            let Some(inner) = rest.get(..close) else {
                break;
            };
            if has_top_level_pipe(inner) {
                out.push(format!(
                    "Makefile:{line_no}: $(shell ...) has a top-level pipe; make's shell function ignores exit statuses, so the left command's failure is swallowed (spec/05 section 2)"
                ));
            }
            cursor = start.saturating_add(close).saturating_add(1);
        }
    }
    out
}

#[must_use]
pub fn text_violations(text: &str) -> Vec<String> {
    let recipes = logical_recipe_lines(text);
    let mut out = Vec::new();
    for (line_no, line) in &recipes {
        out.extend(semicolon_violations(*line_no, line));
    }
    out.extend(shell_pipefail_violations(text, &recipes));
    out.extend(make_shell_calls(text));
    out
}

#[must_use]
pub fn violations(repo: &Path) -> Vec<String> {
    match read_makefile(repo) {
        Ok(text) => text_violations(&text),
        Err(e) => vec![e],
    }
}

#[cfg(test)]
#[path = "makefile_recipes_tests.rs"]
mod tests;
