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
        ActionSubject::WriteFile { path, content: _ } => outside_check(path, action, false),
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
        return RuleVerdict::of(MachineLevel::Deny, "bot-machine-outside", false);
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
        ActionSubject::WriteFile { path, content: _ } => user_file_rule(action, path, false),
        ActionSubject::Command {
            line,
            timeout_ms: _,
        } => user_command_rule(line),
    }
}

fn user_file_rule(action: &GateAction, path: &PathView, read_only: bool) -> RuleVerdict {
    if !read_only && !inside_workdir(path, &action.workdir) {
        return RuleVerdict::of(MachineLevel::Deny, "user-machine-outside-write", false);
    }
    RuleVerdict::of(MachineLevel::AskEveryTime, "user-machine-files", read_only)
}

fn user_command_rule(line: &str) -> RuleVerdict {
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
    let base = workdir.trim_end_matches('/');
    let target = path.effective(base);
    target == base || target.starts_with(&format!("{base}/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::ActionSubject;
    use anybot_protocol::{CapState, ComputerCaps, ComputerOwner, ComputerPlatform};

    fn bot_caps() -> ComputerCaps {
        ComputerCaps {
            owner: ComputerOwner::Bot,
            platform: ComputerPlatform::Linux,
            run_commands: CapState::Supported,
            read_files: CapState::Supported,
            write_files: CapState::Supported,
            check_path: CapState::Supported,
            command_sandbox: CapState::Supported,
            ..ComputerCaps::declared(ComputerPlatform::Linux, ComputerOwner::Bot)
        }
    }

    fn user_caps() -> ComputerCaps {
        ComputerCaps {
            owner: ComputerOwner::User,
            platform: ComputerPlatform::Mac,
            run_commands: CapState::Supported,
            read_files: CapState::Supported,
            write_files: CapState::Supported,
            check_path: CapState::Supported,
            command_sandbox: CapState::Supported,
            ..ComputerCaps::declared(ComputerPlatform::Mac, ComputerOwner::User)
        }
    }

    fn shell(line: &str, workdir: &str) -> GateAction {
        GateAction::command("shell", line, workdir, Vec::new())
    }

    fn read(path: &str) -> GateAction {
        GateAction {
            tool: "read_file".to_owned(),
            subject: ActionSubject::ReadFile {
                path: PathView::raw(path),
            },
            extra_paths: Vec::new(),
            workdir: "/workspace".to_owned(),
        }
    }

    #[test]
    fn test_machine_rules_bot_machine_reads_writes_and_deletes_in_own_folder() {
        let caps = bot_caps();
        let write = GateAction {
            tool: "edit_file".to_owned(),
            subject: ActionSubject::WriteFile {
                path: PathView::raw("notes.md"),
                content: String::new(),
            },
            extra_paths: Vec::new(),
            workdir: "/workspace".to_owned(),
        };
        assert_eq!(default_rule(&write, &caps).rule, "bot-machine-files");
        let delete = GateAction::command(
            "shell",
            "rm notes.md",
            "/workspace",
            vec![PathView::raw("notes.md")],
        );
        let verdict = default_rule(&delete, &caps);
        assert_eq!(verdict.level, MachineLevel::Direct);
        assert_eq!(verdict.rule, "bot-machine-own-folder");
        let outside = GateAction::command(
            "shell",
            "rm /etc/hosts",
            "/workspace",
            vec![PathView::raw("/etc/hosts")],
        );
        assert_eq!(default_rule(&outside, &caps).rule, "bot-machine-outside");
        assert_eq!(
            default_rule(&read("/workspace/notes.md"), &caps).rule,
            "bot-machine-files"
        );
    }

    #[test]
    fn test_machine_rules_bot_machine_denies_outside_and_asks_for_outbound() {
        let caps = bot_caps();
        assert_eq!(
            default_rule(&read("/etc/passwd"), &caps).rule,
            "bot-machine-outside"
        );
        let send = shell("git push origin main", "/workspace");
        let verdict = default_rule(&send, &caps);
        assert_eq!(verdict.level, MachineLevel::AskEveryTime);
        assert_eq!(verdict.rule, "bot-machine-sending");
        let shell_verdict = default_rule(&shell("ls", "/workspace"), &caps);
        assert_eq!(shell_verdict.level, MachineLevel::Direct);
    }

    #[test]
    fn test_machine_rules_user_machine_splits_read_only_chains_and_file_tools() {
        let caps = user_caps();
        let direct = default_rule(&shell("ls -la", "/workspace"), &caps);
        assert_eq!(direct.level, MachineLevel::Direct);
        assert!(direct.read_only);
        let ask = default_rule(
            &shell("cd /workspace && cat notes.txt", "/workspace"),
            &caps,
        );
        assert_eq!(ask.level, MachineLevel::Ask);
        let write = GateAction {
            tool: "edit_file".to_owned(),
            subject: ActionSubject::WriteFile {
                path: PathView::raw("/etc/hosts"),
                content: String::new(),
            },
            extra_paths: Vec::new(),
            workdir: "/workspace".to_owned(),
        };
        assert_eq!(default_rule(&write, &caps).level, MachineLevel::Deny);
        assert_eq!(
            default_rule(&read("notes.md"), &caps).level,
            MachineLevel::AskEveryTime
        );
    }

    #[test]
    fn test_machine_rules_user_machine_without_sandbox_asks_every_time() {
        let caps = ComputerCaps {
            command_sandbox: CapState::Unsupported,
            ..user_caps()
        };
        let verdict = default_rule(&shell("ls", "/workspace"), &caps);
        assert_eq!(verdict.level, MachineLevel::AskEveryTime);
        assert_eq!(verdict.rule, "user-machine-no-sandbox");
    }
}
