use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ActionClass {
    Delete,
    SendToOthers,
    Share,
    Publish,
    Payment,
    PasswordChange,
}

impl fmt::Display for ActionClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            ActionClass::Delete => "delete",
            ActionClass::SendToOthers => "send",
            ActionClass::Share => "share",
            ActionClass::Publish => "publish",
            ActionClass::Payment => "payment",
            ActionClass::PasswordChange => "password",
        };
        f.write_str(text)
    }
}

pub const ALL_CLASSES: [ActionClass; 6] = [
    ActionClass::Delete,
    ActionClass::SendToOthers,
    ActionClass::Share,
    ActionClass::Publish,
    ActionClass::Payment,
    ActionClass::PasswordChange,
];

fn class_words(class: ActionClass) -> &'static [&'static str] {
    match class {
        ActionClass::Delete => &["delete", "remove", "trash", "erase", "destroy", "purge"],
        ActionClass::SendToOthers => &["send", "message", "reply", "comment", "email"],
        ActionClass::Share => &["share", "invite", "permission"],
        ActionClass::Publish => &["publish", "post", "tweet"],
        ActionClass::Payment => &[
            "pay", "charge", "purchase", "checkout", "transfer", "refund",
        ],
        ActionClass::PasswordChange => &["password", "passwd"],
    }
}

fn command_words(class: ActionClass) -> (&'static [&'static str], &'static [&'static str]) {
    match class {
        ActionClass::Delete => (
            &["rm", "rmdir", "unlink", "shred", "trash"],
            &["clean", "rm"],
        ),
        ActionClass::SendToOthers => (&["mail", "sendmail", "mutt"], &["send", "mail"]),
        ActionClass::Share => (&["chmod", "setfacl"], &["share", "grant"]),
        ActionClass::Publish => (
            &["twine"],
            &["push", "publish", "upload", "release", "create"],
        ),
        ActionClass::Payment => (&["pay", "charge"], &["pay", "charge"]),
        ActionClass::PasswordChange => (&["passwd", "htpasswd", "chpasswd"], &["passwd", "set"]),
    }
}

fn click_words(class: ActionClass) -> &'static [&'static str] {
    match class {
        ActionClass::Delete => &[
            "delete",
            "remove",
            "clear",
            "format",
            "unsubscribe",
            "sign out",
            "log out",
        ],
        ActionClass::SendToOthers => &["send", "forward", "mail"],
        ActionClass::Share => &["share", "invite"],
        ActionClass::Publish => &["publish", "post", "submit", "send", "tweet"],
        ActionClass::Payment => &[
            "pay",
            "purchase",
            "buy now",
            "checkout",
            "place order",
            "confirm order",
            "transfer",
        ],
        ActionClass::PasswordChange => &["change password", "reset password"],
    }
}

#[must_use]
pub fn word_hit(name: &str, word: &str) -> bool {
    name.to_lowercase()
        .split(['_', '-', '.', ' '])
        .any(|part| part == word)
}

#[must_use]
pub fn classes_of_connector_tool(name: &str, destructive: bool) -> Vec<ActionClass> {
    let mut out = vec![];
    if destructive {
        out.push(ActionClass::Delete);
    }
    for class in ALL_CLASSES {
        if class_words(class).iter().any(|w| word_hit(name, w)) {
            out.push(class);
        }
    }
    out.sort();
    out.dedup();
    out
}

const GIT_VALUE_OPTIONS: [&str; 6] = [
    "-C",
    "-c",
    "--git-dir",
    "--work-tree",
    "--namespace",
    "--exec-path",
];

#[must_use]
pub fn classes_of_command(line: &str) -> Vec<ActionClass> {
    let crate::command_read::ReadLine::Read(commands) = crate::command_read::read_line(line) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for command in commands {
        if command.name == "git" && git_alias(&command.args) {
            continue;
        }
        let pair = command_pair(&command);
        for class in ALL_CLASSES {
            if class_hit(class, &command, &pair) {
                out.push(class);
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

fn class_hit(
    class: ActionClass,
    command: &crate::command_read::ReadCommand,
    pair: &[String],
) -> bool {
    let (firsts, seconds) = command_words(class);
    firsts
        .iter()
        .any(|word| pair.first().is_some_and(|token| token == word))
        || seconds
            .iter()
            .any(|word| pair.get(1).is_some_and(|token| token == word))
        || (class == ActionClass::Delete && command.args.iter().any(|arg| arg == "-delete"))
}

fn command_pair(command: &crate::command_read::ReadCommand) -> Vec<String> {
    if command.name != "git" {
        let second = command.args.first().cloned().unwrap_or_default();
        return vec![command.name.clone(), second];
    }
    vec![command.name.clone(), git_subcommand(&command.args)]
}

fn git_alias(args: &[String]) -> bool {
    args.windows(2).any(|pair| {
        pair.first()
            .is_some_and(|flag| flag == "-c" || flag.starts_with("-c"))
            && pair
                .get(1)
                .is_some_and(|value| value.to_ascii_lowercase().starts_with("alias."))
    }) || args.iter().any(|arg| {
        arg.to_ascii_lowercase()
            .strip_prefix("-c")
            .is_some_and(|rest| rest.trim_start_matches('=').starts_with("alias."))
    })
}

pub(crate) fn git_subcommand(args: &[String]) -> String {
    let mut value_expected = false;
    for token in args {
        if value_expected {
            value_expected = false;
            continue;
        }
        if token == "--" {
            break;
        }
        if let Some(flag) = token.strip_prefix('-') {
            if flag.is_empty() {
                break;
            }
            if GIT_VALUE_OPTIONS.contains(&token.as_str()) && !token.contains('=') {
                value_expected = true;
            }
            if token == "-c" || (token.starts_with("-c") && !token.contains('=')) {
                value_expected = true;
            }
            continue;
        }
        return token.clone();
    }
    String::new()
}

pub(crate) fn egress_tokens(segment: &str) -> Vec<String> {
    let mut parts = segment.split_whitespace();
    let head = parts.next().unwrap_or_default().to_owned();
    if head != "git" {
        let second = parts.next().unwrap_or_default().to_owned();
        return vec![head, second];
    }
    let mut subcommand = String::new();
    let mut value_expected = false;
    for token in parts {
        if value_expected {
            value_expected = false;
            continue;
        }
        if token.starts_with('-') {
            if GIT_VALUE_OPTIONS.contains(&token) && !token.contains('=') {
                value_expected = true;
            }
            continue;
        }
        token.clone_into(&mut subcommand);
        break;
    }
    vec![head, subcommand]
}

#[must_use]
pub fn classes_of_click_label(label: &str) -> Vec<ActionClass> {
    let text = label.to_lowercase();
    ALL_CLASSES
        .iter()
        .filter(|class| {
            let class = **class;
            click_words(class)
                .iter()
                .any(|w| text == *w || word_hit(label, w))
        })
        .copied()
        .collect()
}
