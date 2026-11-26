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

const GIT_GLOBAL_OPTIONS: [&str; 10] = [
    "-C",
    "-c",
    "--git-dir",
    "--work-tree",
    "--namespace",
    "--exec-path",
    "--bare",
    "--config",
    "--paginate",
    "--no-pager",
];

#[must_use]
pub fn classes_of_command(line: &str) -> Vec<ActionClass> {
    let mut out = Vec::new();
    for segment in crate::sandbox_deny::split_segments(line) {
        let inner = crate::sandbox_deny::strip_privilege_wrappers(
            &crate::sandbox_deny::strip_wrappers(&segment),
        );
        let lower: Vec<String> = egress_tokens(&inner)
            .iter()
            .map(|t| t.to_lowercase())
            .collect();
        for class in ALL_CLASSES {
            let (firsts, seconds) = command_words(class);
            let hit = firsts.iter().any(|w| lower.first().is_some_and(|t| t == w))
                || seconds.iter().any(|w| lower.get(1).is_some_and(|t| t == w))
                || (class == ActionClass::Delete && segment.contains(" -delete"));
            if hit {
                out.push(class);
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

fn egress_tokens(segment: &str) -> Vec<String> {
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
        if GIT_GLOBAL_OPTIONS.contains(&token) && !token.contains('=') {
            if token == "-C"
                || token == "-c"
                || token == "--git-dir"
                || token == "--work-tree"
                || token == "--namespace"
                || token == "--exec-path"
            {
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
