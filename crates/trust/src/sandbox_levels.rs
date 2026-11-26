use crate::sandbox_deny::split_segments;

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
    if head == "git"
        && segment
            .split_whitespace()
            .nth(1)
            .is_some_and(|sub| READ_ONLY_GIT_SUBCOMMANDS.contains(&sub))
    {
        return true;
    }
    false
}

#[must_use]
pub fn command_level(line: &str) -> CommandLevel {
    let segments = split_segments(line);
    let joined = segments.join(" ");
    if has_unbalanced_shape(&joined) || has_hidden_subcommand(&joined) {
        return CommandLevel::AskEveryTime;
    }
    if segments.iter().any(|s| is_interactive_shell(s)) {
        return CommandLevel::AskEveryTime;
    }
    if segments.iter().any(|s| writes_new_file(s)) {
        return CommandLevel::AskEveryTime;
    }
    if has_redirect(&joined) {
        return CommandLevel::AskEveryTime;
    }
    if segments.iter().all(|s| is_read_only_segment(s)) {
        return CommandLevel::Direct;
    }
    if segments.iter().filter(|s| !is_read_only_segment(s)).count() >= 2 {
        return CommandLevel::AskEveryTime;
    }
    if segments.iter().any(|s| edits_in_place(s)) {
        return CommandLevel::Ask;
    }
    CommandLevel::Ask
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
