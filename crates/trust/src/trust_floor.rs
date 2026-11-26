use std::fmt;

use crate::gate_engine::GateLayer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloorDeny {
    DeniedCommand,
    UnreadableCommand,
    NoSandbox,
    PrivateFile,
    AnyBotWindow,
    CloudMetadata,
}

impl fmt::Display for FloorDeny {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            FloorDeny::DeniedCommand => "sandbox-deny",
            FloorDeny::UnreadableCommand => crate::command_read::CODE_TOOL_UNREADABLE_SHAPE,
            FloorDeny::NoSandbox => "computer-no-sandbox",
            FloorDeny::PrivateFile => "private-file",
            FloorDeny::AnyBotWindow => "cannot-operate-anybot-window",
            FloorDeny::CloudMetadata => "cloud-metadata",
        };
        f.write_str(text)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FloorAction {
    Command {
        line: String,
        bot_machine: bool,
        has_sandbox: bool,
    },
    FilePath {
        path: String,
    },
    ClickTarget {
        app: String,
        label: String,
    },
    Url {
        host: String,
        port: u16,
    },
}

const PRIVATE_PATH_PREFIXES: [&str; 8] = [
    "~/.ssh",
    "~/.aws",
    "~/.gnupg",
    "/etc/shadow",
    "/etc/sudoers",
    "/Library/Keychains",
    "/.securebookmarkswrite",
    "cookies",
];

const CLOUD_METADATA_HOSTS: [&str; 2] = ["169.254.169.254", "metadata.google.internal"];

#[must_use]
pub fn is_denied_command(line: &str, bot_machine: bool) -> bool {
    crate::sandbox_deny::deny_token(line, bot_machine).is_some()
}

#[must_use]
pub fn is_private_path(path: &str) -> bool {
    let text = path.to_lowercase();
    PRIVATE_PATH_PREFIXES.iter().any(|p| text.contains(p))
}

#[must_use]
pub fn is_anybot_window(app: &str) -> bool {
    app.eq_ignore_ascii_case("anybot")
}

#[must_use]
pub fn is_cloud_metadata(host: &str) -> bool {
    CLOUD_METADATA_HOSTS
        .iter()
        .any(|h| host.eq_ignore_ascii_case(h))
}

#[must_use]
pub fn floor_check(action: &FloorAction) -> Option<FloorDeny> {
    match action {
        FloorAction::Command {
            line,
            bot_machine,
            has_sandbox,
        } => floor_command_check(line, *bot_machine, *has_sandbox),
        FloorAction::FilePath { path } => {
            if is_private_path(path) {
                Some(FloorDeny::PrivateFile)
            } else {
                None
            }
        }
        FloorAction::ClickTarget { app, label: _ } => {
            if is_anybot_window(app) {
                Some(FloorDeny::AnyBotWindow)
            } else {
                None
            }
        }
        FloorAction::Url { host, port: _ } => {
            if is_cloud_metadata(host) {
                Some(FloorDeny::CloudMetadata)
            } else {
                None
            }
        }
    }
}

#[must_use]
fn floor_command_check(line: &str, bot_machine: bool, has_sandbox: bool) -> Option<FloorDeny> {
    if is_denied_command(line, bot_machine) {
        return Some(FloorDeny::DeniedCommand);
    }
    if crate::sandbox_peel::unreadable_reason(line).is_some() {
        return Some(FloorDeny::UnreadableCommand);
    }
    if !has_sandbox {
        return Some(FloorDeny::NoSandbox);
    }
    None
}

#[must_use]
pub fn floor_layer() -> GateLayer {
    GateLayer::Floor
}
