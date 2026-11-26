use crate::action::{ActionSubject, GateAction, PathView};
use crate::machine_rules::{MachineLevel, default_rule};
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
    let outside_verdict = default_rule(&outside, &caps);
    assert_eq!(outside_verdict.rule, "bot-machine-outside-delete");
    assert_eq!(outside_verdict.level, MachineLevel::AskEveryTime);
    assert_eq!(
        default_rule(&read("/workspace/notes.md"), &caps).rule,
        "bot-machine-files"
    );
}

fn delete_action(line: &str, targets: Vec<PathView>) -> GateAction {
    GateAction::command("shell", line, "/workspace", targets)
}

#[test]
fn test_a_delete_with_any_outside_target_is_not_the_own_folder() {
    let caps = bot_caps();
    let mixed = delete_action(
        "rm notes.md /etc/passwd",
        vec![PathView::raw("notes.md"), PathView::raw("/etc/passwd")],
    );
    let verdict = default_rule(&mixed, &caps);
    assert_eq!(verdict.level, MachineLevel::AskEveryTime);
    assert_eq!(verdict.rule, "bot-machine-outside-delete");
}

#[test]
fn test_a_delete_with_expanding_tokens_is_not_the_own_folder() {
    let caps = bot_caps();
    let starred = delete_action("rm build/*.o", vec![PathView::unresolvable("build/*.o")]);
    let tilde = delete_action("rm ~/notes.md", vec![PathView::unresolvable("~/notes.md")]);
    let dollar = delete_action(
        "rm $HOME/notes.md",
        vec![PathView::unresolvable("$HOME/notes.md")],
    );
    for action in [&starred, &tilde, &dollar] {
        let verdict = default_rule(action, &caps);
        assert_eq!(verdict.level, MachineLevel::AskEveryTime);
    }
}

#[test]
fn test_a_delete_keeps_the_letters_of_its_paths() {
    let caps = bot_caps();
    let mixed_case = delete_action("rm Notes.MD", vec![PathView::raw("Notes.MD")]);
    let outside_case = delete_action("rm /ETC/hosts", vec![PathView::raw("/ETC/hosts")]);
    assert_eq!(
        default_rule(&mixed_case, &caps).rule,
        "bot-machine-own-folder"
    );
    assert_eq!(
        default_rule(&outside_case, &caps).rule,
        "bot-machine-outside-delete"
    );
}

#[test]
fn test_a_delete_with_an_unresolvable_symlink_is_not_the_own_folder() {
    let caps = bot_caps();
    let broken = delete_action("rm link.txt", vec![PathView::unresolvable("link.txt")]);
    let verdict = default_rule(&broken, &caps);
    assert_eq!(verdict.level, MachineLevel::AskEveryTime);
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

#[test]
fn test_a_wrapped_push_asks_on_the_bot_machine() {
    let verdict = default_rule(
        &shell("sudo nohup git push origin", "/workspace"),
        &bot_caps(),
    );
    assert_eq!(verdict.level, MachineLevel::AskEveryTime);
    assert_eq!(verdict.rule, "bot-machine-sending");
}

#[test]
fn test_an_unreadable_command_asks_every_time_on_the_bot_machine() {
    let verdict = default_rule(&shell("eval ls", "/workspace"), &bot_caps());
    assert_eq!(verdict.level, MachineLevel::AskEveryTime);
    assert_eq!(verdict.rule, "tool-unreadable-shape");
}

#[test]
fn test_an_unreadable_command_asks_every_time_on_the_user_machine() {
    let verdict = default_rule(&shell("bash << EOF", "/workspace"), &user_caps());
    assert_eq!(verdict.level, MachineLevel::AskEveryTime);
    assert_eq!(verdict.rule, "tool-unreadable-shape");
}

fn delete_command(line: &str) -> GateAction {
    let request = crate::pipeline::ActionRequest {
        context: crate::evidence::ActionContext {
            task: anybot_foundation::TaskId::generate(&anybot_foundation::SystemClock),
            turn: anybot_foundation::TurnId::generate(&anybot_foundation::SystemClock),
            evidence_no: 1,
            computer: "bot-1".to_owned(),
            workdir: "/workspace".to_owned(),
            tool: "shell".to_owned(),
            classes: Vec::new(),
        },
        subject: crate::pipeline::RequestedAction::Command {
            line: line.to_owned(),
            timeout_ms: 1000,
        },
    };
    GateAction::command("shell", line, "/workspace", request.delete_targets())
}

#[test]
fn test_a_cd_outside_the_workspace_moves_the_delete_target_out() {
    let action = delete_command("cd /tmp && rm -rf build");
    let verdict = default_rule(&action, &bot_caps());
    assert_eq!(verdict.level, MachineLevel::AskEveryTime);
    assert_eq!(verdict.rule, "bot-machine-outside-delete");
}

#[test]
fn test_an_unreadable_cd_target_makes_the_delete_ask() {
    let action = delete_command("cd $HOME && rm notes.md");
    let verdict = default_rule(&action, &bot_caps());
    assert_eq!(verdict.level, MachineLevel::AskEveryTime);
    assert_eq!(verdict.rule, "tool-unreadable-shape");
}
