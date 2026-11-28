use crate::always_list::AlwaysList;
use crate::command_key::command_action_key;
use crate::sandbox_peel::visible_segments;

pub const READ_ONLY_COMMANDS: [&str; 20] = [
    "ls", "pwd", "cat", "head", "tail", "wc", "echo", "rg", "grep", "find", "file", "stat", "du",
    "df", "date", "which", "tree", "sort", "uniq", "diff",
];

const READ_ONLY_GIT_SUBCOMMANDS: [&str; 4] = ["status", "diff", "log", "show"];

const INTERACTIVE_SHELLS: [&str; 9] = [
    "bash",
    "sh",
    "zsh",
    "fish",
    "ksh",
    "tcsh",
    "csh",
    "powershell",
    "cmd",
];

const NEW_FILE_OPTIONS: [(&str, &str); 4] = [
    ("sort", "-o"),
    ("find", "-delete"),
    ("tee", "-o"),
    ("dd", "of="),
];

const IN_PLACE_OPTIONS: [(&str, &str); 3] = [("sed", "-i"), ("perl", "-i"), ("awk", "-i")];

const REDIRECT_TOKENS: [&str; 3] = [">", ">>", "1>"];

const HIDDEN_SUBCOMMAND_TOKENS: [&str; 3] = ["$(", "`", "eval"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandLevel {
    Direct,
    Ask,
    AskEveryTime,
}

#[must_use]
pub fn is_read_only_segment(segment: &str) -> bool {
    let Some(head) = segment.split_whitespace().next() else {
        return false;
    };
    if READ_ONLY_COMMANDS.contains(&head) {
        return true;
    }
    if head == "git" {
        let tokens = crate::safety_n5::egress_tokens(segment);
        let sub = tokens.get(1).map(String::as_str).unwrap_or_default();
        return READ_ONLY_GIT_SUBCOMMANDS.contains(&sub);
    }
    false
}

#[must_use]
pub fn command_level(line: &str) -> CommandLevel {
    command_level_with_always(line, &AlwaysList::default(), "")
}

#[must_use]
pub fn command_level_with_always(line: &str, always: &AlwaysList, computer: &str) -> CommandLevel {
    if has_ask_every_time_hazard(line) {
        return CommandLevel::AskEveryTime;
    }
    let segments = visible_segments(line);
    if segments.iter().all(|s| is_read_only_segment(s)) {
        return CommandLevel::Direct;
    }
    if segments
        .iter()
        .filter(|s| !is_read_only_segment(s) && !segment_in_always(s, always, computer))
        .count()
        >= 2
    {
        return CommandLevel::AskEveryTime;
    }
    if segments.iter().any(|s| edits_in_place(s)) {
        return CommandLevel::Ask;
    }
    CommandLevel::Ask
}

#[must_use]
pub(crate) fn has_ask_every_time_hazard(line: &str) -> bool {
    if crate::sandbox_peel::unreadable_reason(line).is_some() {
        return true;
    }
    let segments = visible_segments(line);
    let joined = segments.join(" ");
    has_unbalanced_shape(&joined)
        || has_hidden_subcommand(&joined)
        || segments.iter().any(|s| is_interactive_shell(s))
        || segments.iter().any(|s| writes_new_file(s))
        || has_redirect(&joined)
}

#[must_use]
pub(crate) fn segment_in_always(segment: &str, always: &AlwaysList, computer: &str) -> bool {
    if computer.is_empty() {
        return false;
    }
    let key = format!("command {} on {computer}", command_action_key(segment));
    always.contains(&key)
}

#[must_use]
pub(crate) fn all_segments_always_covered(line: &str, always: &AlwaysList, computer: &str) -> bool {
    if has_ask_every_time_hazard(line) {
        return false;
    }
    let segments = visible_segments(line);
    if segments.is_empty() {
        return false;
    }
    let all_ok = segments
        .iter()
        .all(|s| is_read_only_segment(s) || segment_in_always(s, always, computer));
    let any_always = segments
        .iter()
        .any(|s| segment_in_always(s, always, computer));
    all_ok && any_always
}

fn is_interactive_shell(segment: &str) -> bool {
    segment
        .split_whitespace()
        .next()
        .is_some_and(|head| INTERACTIVE_SHELLS.contains(&head))
}

fn writes_new_file(segment: &str) -> bool {
    let Some(head) = segment.split_whitespace().next() else {
        return false;
    };
    NEW_FILE_OPTIONS
        .iter()
        .any(|(cmd, opt)| head == *cmd && segment.contains(*opt))
}

fn edits_in_place(segment: &str) -> bool {
    let Some(head) = segment.split_whitespace().next() else {
        return false;
    };
    IN_PLACE_OPTIONS
        .iter()
        .any(|(cmd, opt)| head == *cmd && segment.contains(*opt))
}

fn has_redirect(joined: &str) -> bool {
    REDIRECT_TOKENS.iter().any(|t| joined.contains(t))
}

fn has_hidden_subcommand(joined: &str) -> bool {
    HIDDEN_SUBCOMMAND_TOKENS.iter().any(|t| joined.contains(t))
}

#[must_use]
pub fn has_unbalanced_shape(text: &str) -> bool {
    let quotes_odd = count_char(text, '\'') % 2 == 1 || count_char(text, '"') % 2 == 1;
    let parens_odd = count_char(text, '(') % 2 == 1 || count_char(text, ')') % 2 == 1;
    quotes_odd || parens_odd
}

fn count_char(text: &str, needle: char) -> usize {
    text.chars().filter(|c| *c == needle).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_sandbox_levels_1() {
        const EXPECTED: [(&str, CommandLevel); 27] = [
            ("ls", CommandLevel::Direct),
            ("pwd", CommandLevel::Direct),
            ("cat", CommandLevel::Direct),
            ("head", CommandLevel::Direct),
            ("tail", CommandLevel::Direct),
            ("wc", CommandLevel::Direct),
            ("echo", CommandLevel::Direct),
            ("rg", CommandLevel::Direct),
            ("grep", CommandLevel::Direct),
            ("find", CommandLevel::Direct),
            ("file", CommandLevel::Direct),
            ("stat", CommandLevel::Direct),
            ("du", CommandLevel::Direct),
            ("df", CommandLevel::Direct),
            ("date", CommandLevel::Direct),
            ("which", CommandLevel::Direct),
            ("tree", CommandLevel::Direct),
            ("sort", CommandLevel::Direct),
            ("uniq", CommandLevel::Direct),
            ("diff", CommandLevel::Direct),
            ("git status", CommandLevel::Direct),
            ("git diff", CommandLevel::Direct),
            ("git log", CommandLevel::Direct),
            ("git show", CommandLevel::Direct),
            ("ls && git status", CommandLevel::Direct),
            ("ls > out.txt", CommandLevel::AskEveryTime),
            ("grep x | sort -o y", CommandLevel::AskEveryTime),
        ];
        for (line, level) in EXPECTED {
            assert_eq!(command_level(line), level, "line {line}");
        }
    }

    #[test]
    fn test_sandbox_levels_flags_shells_subshells_and_long_chains() {
        assert_eq!(command_level("bash"), CommandLevel::AskEveryTime);
        assert_eq!(command_level("ls $(whoami)"), CommandLevel::AskEveryTime);
        assert_eq!(command_level("eval ls"), CommandLevel::AskEveryTime);
        assert_eq!(
            command_level("ls && make test && npm run x"),
            CommandLevel::AskEveryTime
        );
        assert_eq!(command_level("sed -i s/a/b/ notes.txt"), CommandLevel::Ask);
        assert_eq!(
            command_level("cd /workspace && cat notes.txt"),
            CommandLevel::Ask
        );
        assert_eq!(command_level("echo hello"), CommandLevel::Direct);
    }

    #[test]
    fn test_sandbox_levels_unbalanced_quotes_count_as_indeterminate() {
        assert_eq!(command_level("echo \"abc"), CommandLevel::AskEveryTime);
        assert_eq!(command_level("echo 'abc'"), CommandLevel::Direct);
    }
}
