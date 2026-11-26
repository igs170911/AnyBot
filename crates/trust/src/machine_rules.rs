use crate::action::{ActionSubject, GateAction, PathView};
use crate::safety_n5::ActionClass;
use crate::sandbox_levels::{CommandLevel, command_level};
use anybot_protocol::{ComputerCaps, ComputerOwner};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineLevel {
    Direct,
    Ask,
    AskEveryTime,
    Deny,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleVerdict {
    pub level: MachineLevel,
    pub rule: &'static str,
    pub read_only: bool,
}

impl RuleVerdict {
    fn of(level: MachineLevel, rule: &'static str, read_only: bool) -> Self {
        RuleVerdict {
            level,
            rule,
            read_only,
        }
    }

    #[must_use]
    pub fn opinion(&self) -> crate::gate_engine::Opinion {
        match self.level {
            MachineLevel::Direct => crate::gate_engine::Opinion::Allow,
            MachineLevel::Ask | MachineLevel::AskEveryTime => crate::gate_engine::Opinion::Ask,
            MachineLevel::Deny => crate::gate_engine::Opinion::Deny,
        }
    }
}

#[must_use]
pub fn default_rule(action: &GateAction, caps: &ComputerCaps) -> RuleVerdict {
    if caps.owner == ComputerOwner::Bot {
        return bot_machine_rule(action);
    }
    user_machine_rule(action, caps)
}

fn bot_machine_rule(action: &GateAction) -> RuleVerdict {
    match &action.subject {
        ActionSubject::Capabilities => {
            RuleVerdict::of(MachineLevel::Direct, "bot-machine-caps", true)
        }
        ActionSubject::PathCheck { .. } => {
            RuleVerdict::of(MachineLevel::Direct, "bot-machine-files", true)
        }
        ActionSubject::ReadFile { path } => outside_check(path, action, true),
        ActionSubject::WriteFile { path, content: _ } | ActionSubject::EditFile { path, .. } => {
            outside_check(path, action, false)
        }
        ActionSubject::Command {
            line,
            timeout_ms: _,
        } => bot_command_rule(action, line),
    }
}

fn outside_check(path: &PathView, action: &GateAction, read_only: bool) -> RuleVerdict {
    if inside_workdir(path, &action.workdir) {
        return RuleVerdict::of(MachineLevel::Direct, "bot-machine-files", read_only);
    }
    RuleVerdict::of(MachineLevel::Deny, "bot-machine-outside", read_only)
}

fn bot_command_rule(action: &GateAction, line: &str) -> RuleVerdict {
    if let Some(code) = crate::sandbox_peel::unreadable_reason(line) {
        return RuleVerdict::of(MachineLevel::AskEveryTime, code, false);
    }
    let classes = crate::safety_n5::classes_of_command(line);
    if classes.iter().copied().any(is_outbound_class) {
        return RuleVerdict::of(MachineLevel::AskEveryTime, "bot-machine-sending", false);
    }
    if classes.contains(&ActionClass::Delete) {
        let targets = delete_targets(action);
        if targets.is_empty() {
            return RuleVerdict::of(
                MachineLevel::AskEveryTime,
                "bot-machine-delete-unknown",
                false,
            );
        }
        if targets.iter().all(|p| inside_workdir(p, &action.workdir)) {
            return RuleVerdict::of(MachineLevel::Direct, "bot-machine-own-folder", false);
        }
        return RuleVerdict::of(
            MachineLevel::AskEveryTime,
            "bot-machine-outside-delete",
            false,
        );
    }
    RuleVerdict::of(MachineLevel::Direct, "bot-machine-command", false)
}

fn is_outbound_class(class: ActionClass) -> bool {
    matches!(
        class,
        ActionClass::SendToOthers
            | ActionClass::Share
            | ActionClass::Publish
            | ActionClass::Payment
            | ActionClass::PasswordChange
    )
}

fn delete_targets(action: &GateAction) -> Vec<&PathView> {
    action.path_views()
}

fn user_machine_rule(action: &GateAction, caps: &ComputerCaps) -> RuleVerdict {
    if !caps.command_sandbox.yes() {
        return RuleVerdict::of(MachineLevel::AskEveryTime, "user-machine-no-sandbox", false);
    }
    match &action.subject {
        ActionSubject::Capabilities => {
            RuleVerdict::of(MachineLevel::Direct, "user-machine-caps", true)
        }
        ActionSubject::PathCheck { path } | ActionSubject::ReadFile { path } => {
            user_file_rule(action, path, true)
        }
        ActionSubject::WriteFile { path, content: _ } | ActionSubject::EditFile { path, .. } => {
            user_file_rule(action, path, false)
        }
        ActionSubject::Command {
            line,
            timeout_ms: _,
        } => user_command_rule(line),
    }
}

fn user_file_rule(action: &GateAction, path: &PathView, read_only: bool) -> RuleVerdict {
    if !read_only && !inside_workdir(path, &action.workdir) {
        return RuleVerdict::of(
            MachineLevel::AskEveryTime,
            "user-machine-outside-write",
            false,
        );
    }
    RuleVerdict::of(MachineLevel::AskEveryTime, "user-machine-files", read_only)
}

fn user_command_rule(line: &str) -> RuleVerdict {
    if let Some(code) = crate::sandbox_peel::unreadable_reason(line) {
        return RuleVerdict::of(MachineLevel::AskEveryTime, code, false);
    }
    let classes = crate::safety_n5::classes_of_command(line);
    if classes.iter().copied().any(is_outbound_class) || classes.contains(&ActionClass::Delete) {
        return RuleVerdict::of(MachineLevel::AskEveryTime, "safety-n5-outbound", false);
    }
    match command_level(line) {
        CommandLevel::Direct => {
            RuleVerdict::of(MachineLevel::Direct, "user-machine-read-only", true)
        }
        CommandLevel::AskEveryTime => RuleVerdict::of(
            MachineLevel::AskEveryTime,
            "user-machine-ask-every-time",
            false,
        ),
        CommandLevel::Ask => RuleVerdict::of(MachineLevel::Ask, "user-machine-command", false),
    }
}

#[must_use]
pub fn inside_workdir(path: &PathView, workdir: &str) -> bool {
    if !path.is_resolvable() {
        return false;
    }
    let base = workdir.trim_end_matches('/');
    let target = path.effective(base);
    target == base || target.starts_with(&format!("{base}/"))
}
