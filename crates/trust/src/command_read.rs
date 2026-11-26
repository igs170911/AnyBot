use crate::command_lex::{LexLine, LexToken, LexWord, lex_line};
use crate::command_wrappers::{peel_known_wrapper, wrapper_index};

pub const CODE_TOOL_UNREADABLE_SHAPE: &str = crate::command_lex::CODE_TOOL_UNREADABLE_SHAPE;

pub const SHELL_HEADS: [&str; 6] = ["sh", "bash", "zsh", "ksh", "dash", "fish"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadCommand {
    pub name: String,
    pub args: Vec<String>,
    pub peeled_privilege: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadLine {
    Read(Vec<ReadCommand>),
    Opaque(&'static str),
}

#[must_use]
pub fn read_line(line: &str) -> ReadLine {
    let LexLine::Commands(commands) = lex_line(line) else {
        return ReadLine::Opaque(CODE_TOOL_UNREADABLE_SHAPE);
    };
    let mut out = Vec::new();
    for command in commands {
        match read_command(&command) {
            Some(read) => out.push(read),
            None => return ReadLine::Opaque(CODE_TOOL_UNREADABLE_SHAPE),
        }
    }
    if out.is_empty() {
        return ReadLine::Opaque(CODE_TOOL_UNREADABLE_SHAPE);
    }
    if pipeline_feeds_shell(&out) {
        return ReadLine::Opaque(CODE_TOOL_UNREADABLE_SHAPE);
    }
    ReadLine::Read(out)
}

fn read_command(tokens: &[LexToken]) -> Option<ReadCommand> {
    let words = plain_words(tokens)?;
    let (name, args, peeled_privilege) = peel_wrappers(&words)?;
    if !known_command(&name, &args) {
        return None;
    }
    Some(ReadCommand {
        name,
        args,
        peeled_privilege,
    })
}

fn plain_words(tokens: &[LexToken]) -> Option<Vec<String>> {
    let mut words = Vec::new();
    let mut saw_word = false;
    for token in tokens {
        match token.kind {
            LexWord::Assign if !saw_word => {}
            LexWord::Word | LexWord::Assign => {
                saw_word = true;
                words.push(token.text.to_ascii_lowercase());
            }
            LexWord::Operator | LexWord::Opaque => return None,
        }
    }
    if words.is_empty() { None } else { Some(words) }
}

fn peel_wrappers(words: &[String]) -> Option<(String, Vec<String>, bool)> {
    let mut current = words.to_vec();
    let mut peeled_privilege = false;
    for _ in 0..12 {
        let head = current.first()?;
        let bare = bare_name(head);
        let Some(index) = wrapper_index(&bare) else {
            let name = bare_name(head);
            return Some((
                name,
                current.get(1..).unwrap_or(&[]).to_vec(),
                peeled_privilege,
            ));
        };
        if matches!(bare.as_str(), "sudo" | "doas") {
            peeled_privilege = true;
        }
        let next = peel_known_wrapper(index, current.get(1..).unwrap_or(&[]))?;
        if next == current {
            return None;
        }
        current = next;
    }
    None
}

fn known_command(name: &str, args: &[String]) -> bool {
    if name == "cd" {
        return plain_cd(args);
    }
    if name == "git" && git_alias(args) {
        return false;
    }
    known_head(name)
}

fn plain_cd(args: &[String]) -> bool {
    let Some(path) = args.iter().find(|arg| !arg.starts_with('-')) else {
        return false;
    };
    path != "-" && !path.contains('$') && !path.starts_with('~')
}

fn git_alias(args: &[String]) -> bool {
    args.windows(2).any(|pair| {
        pair.first()
            .is_some_and(|flag| *flag == "-c" || flag.starts_with("-c"))
            && pair
                .get(1)
                .is_some_and(|value| value.to_ascii_lowercase().starts_with("alias."))
    }) || args.iter().any(|arg| {
        arg.to_ascii_lowercase()
            .strip_prefix("-c")
            .is_some_and(|rest| rest.trim_start_matches('=').starts_with("alias."))
    })
}

fn known_head(name: &str) -> bool {
    let bare = name.to_owned();
    if bare.is_empty() || bare.starts_with('-') || wrapper_index(&bare).is_some() {
        return false;
    }
    if name == "cd" {
        return true;
    }
    if SHELL_HEADS.contains(&bare.as_str()) {
        return false;
    }
    if matches!(
        bare.as_str(),
        "eval" | "source" | "." | "alias" | "pushd" | "popd"
    ) {
        return false;
    }
    if bare.starts_with('\\') || bare.starts_with('$') || bare.starts_with('`') {
        return false;
    }
    bare.chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '+'))
}

fn pipeline_feeds_shell(commands: &[ReadCommand]) -> bool {
    commands
        .get(1..)
        .is_some_and(|tail| tail.iter().any(|command| is_shell(&command.name)))
}

fn is_shell(name: &str) -> bool {
    SHELL_HEADS.contains(&bare_name(name).as_str())
}

#[must_use]
pub fn bare_name(head: &str) -> String {
    let trimmed = head.trim_matches(|ch| ch == '\'' || ch == '"');
    let last = trimmed.rsplit(['/', '\\']).next().unwrap_or(trimmed);
    last.to_ascii_lowercase()
}

#[must_use]
pub fn command_text(command: &ReadCommand) -> String {
    if command.args.is_empty() {
        return command.name.clone();
    }
    format!("{} {}", command.name, command.args.join(" "))
}
