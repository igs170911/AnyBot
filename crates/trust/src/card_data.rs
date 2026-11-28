use crate::action::{ActionSubject, GateAction, PathView};
use crate::machine_rules::inside_workdir;
use crate::safety_always::{ActionSpec, concrete_actions};
use crate::safety_n5::classes_of_command;
use anybot_protocol::{CapState, ComputerCaps, ComputerOwner};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardData {
    pub actions: Vec<String>,
    pub classes: Vec<String>,
    pub reason: String,
}

#[must_use]
pub fn build_card(action: &GateAction, caps: &ComputerCaps) -> CardData {
    let specs = card_specs(action, computer_label(caps));
    CardData {
        actions: specs.iter().flat_map(concrete_actions).collect(),
        classes: card_classes(action),
        reason: card_reason(action, caps),
    }
}

fn computer_label(caps: &ComputerCaps) -> String {
    match caps.owner {
        ComputerOwner::Bot => "bot-machine".to_owned(),
        ComputerOwner::User => "user-machine".to_owned(),
        ComputerOwner::Unknown => "unknown".to_owned(),
    }
}

fn card_specs(action: &GateAction, computer: String) -> Vec<ActionSpec> {
    match &action.subject {
        ActionSubject::Command { line, .. } => vec![ActionSpec::Command {
            command: line.clone(),
            computer,
        }],
        ActionSubject::WriteFile { path, content: _ } | ActionSubject::EditFile { path, .. } => {
            vec![ActionSpec::EditFile {
                computer,
                path: file_target(action, path),
            }]
        }
        ActionSubject::ReadFile { path }
        | ActionSubject::ListDir { path }
        | ActionSubject::Search { path, .. }
        | ActionSubject::PathCheck { path } => {
            vec![ActionSpec::ReadFile {
                computer,
                path: file_target(action, path),
            }]
        }
        ActionSubject::Capabilities => Vec::new(),
    }
}

fn file_target(action: &GateAction, path: &PathView) -> String {
    let target = path.effective(&action.workdir);
    if inside_workdir(path, &action.workdir) {
        return target;
    }
    format!("{target} (outside workspace)")
}

fn card_classes(action: &GateAction) -> Vec<String> {
    match &action.subject {
        ActionSubject::Command { line, .. } => classes_of_command(line)
            .iter()
            .map(ToString::to_string)
            .collect(),
        ActionSubject::ReadFile { .. }
        | ActionSubject::ListDir { .. }
        | ActionSubject::Search { .. }
        | ActionSubject::WriteFile { .. }
        | ActionSubject::EditFile { .. }
        | ActionSubject::PathCheck { .. }
        | ActionSubject::Capabilities => Vec::new(),
    }
}

fn card_reason(action: &GateAction, caps: &ComputerCaps) -> String {
    let base = reason_base(action);
    if caps.command_sandbox == CapState::Unsupported {
        return format!("{base}-no-sandbox");
    }
    base.to_owned()
}

fn reason_base(action: &GateAction) -> &'static str {
    match &action.subject {
        ActionSubject::Command { line, .. } if has_hidden_shape(line) => "hidden-subcommand",
        ActionSubject::Command { line, .. } if starts_interactive(line) => "interactive-shell",
        ActionSubject::Command { .. } => "run-command",
        ActionSubject::ReadFile { path }
        | ActionSubject::ListDir { path }
        | ActionSubject::Search { path, .. } => outside_word(path, action, "read"),
        ActionSubject::PathCheck { path } => outside_word(path, action, "path"),
        ActionSubject::WriteFile { path, content: _ } | ActionSubject::EditFile { path, .. } => {
            outside_word(path, action, "write")
        }
        ActionSubject::Capabilities => "read-capabilities",
    }
}

fn outside_word(path: &PathView, action: &GateAction, mode: &str) -> &'static str {
    if inside_workdir(path, &action.workdir) {
        return "file-in-workspace";
    }
    match mode {
        "write" => "write-outside",
        "read" => "read-outside",
        _ => "path-outside",
    }
}

fn has_hidden_shape(line: &str) -> bool {
    line.contains("$(") || line.contains('`') || line.contains("eval")
}

fn starts_interactive(line: &str) -> bool {
    ["bash", "zsh", "fish", "powershell", "cmd", "sh"]
        .iter()
        .any(|shell| line.split_whitespace().next() == Some(*shell))
}
