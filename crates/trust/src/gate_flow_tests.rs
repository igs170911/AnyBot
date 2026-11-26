use crate::action::{ActionSubject, GateAction, PathView};
use crate::always_list::{AlwaysEntry, AlwaysList};
use crate::gate_engine::{GateLayer, Opinion};
use crate::gate_flow::{AdminRules, GateConfig, RuleEntry, UserRules, floor_view, judge};
use crate::gate_full::ModeFlags;
use crate::trust_floor::floor_check;
use anybot_foundation::TimestampMs;
use anybot_protocol::{
    CapState, ComputerCaps, ComputerOwner, ComputerPlatform, GateOutcome, RuleLayer,
};

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

#[test]
fn test_gate_flow_floor_denial_records_the_first_gate() {
    let action = GateAction::command("shell", "mkfs.ext4 /dev/sda1", "/workspace", Vec::new());
    let verdict = judge(&action, &bot_caps(), &GateConfig::plain(), "fake-computer");
    assert_eq!(verdict.outcome, GateOutcome::Deny);
    assert_eq!(verdict.gate_no().value(), 1);
    assert_eq!(verdict.record().layer, RuleLayer::Floor);
}

#[test]
fn test_gate_flow_default_direct_rule_issues_a_ticket_for_that_action() {
    let action = GateAction::command("shell", "ls -la", "/workspace", Vec::new());
    let verdict = judge(&action, &bot_caps(), &GateConfig::plain(), "fake-computer");
    assert_eq!(verdict.outcome, GateOutcome::Allow);
    assert_eq!(verdict.rule, "bot-machine-command");
    let ticket = verdict.ticket(&action, "fake-computer", TimestampMs::from_millis(5));
    assert!(ticket.is_some());
    let other = GateAction::command("shell", "ls -lb", "/workspace", Vec::new());
    let issued = ticket.unwrap();
    assert_eq!(issued.key(), &action.key());
    assert_ne!(issued.key(), &other.key());
    assert_eq!(issued.computer(), "fake-computer");
}

#[test]
fn test_gate_flow_private_path_is_denied_by_the_floor_rule() {
    let action = GateAction {
        tool: "read_file".to_owned(),
        subject: ActionSubject::ReadFile {
            path: PathView::raw("~/.ssh/id_ed25519"),
        },
        extra_paths: Vec::new(),
        workdir: "/workspace".to_owned(),
    };
    let verdict = judge(&action, &bot_caps(), &GateConfig::plain(), "fake-computer");
    assert_eq!(verdict.outcome, GateOutcome::Deny);
    assert!(floor_check(&floor_view(&action, &bot_caps())).is_some());
}

fn config_with(user: UserRules) -> GateConfig {
    GateConfig {
        modes: ModeFlags {
            ask_every_time: false,
            yolo: false,
            direct_mode: false,
            mdm_locks_direct: false,
        },
        user,
        admin: AdminRules::default(),
    }
}

fn git_push_action() -> GateAction {
    GateAction::command("shell", "git push origin main", "/workspace", Vec::new())
}

#[test]
fn test_an_always_list_entry_for_this_computer_allows_the_command() {
    let user = UserRules {
        always: AlwaysList {
            entries: vec![AlwaysEntry {
                action: format!(
                    "command {} on fake-computer",
                    crate::command_key::command_action_key("git push origin main"),
                ),
                added_at: TimestampMs::from_millis(1),
                from_bot: anybot_foundation::BotId::generate(&anybot_foundation::SystemClock),
            }],
        },
        ..UserRules::default()
    };
    let verdict = judge(
        &git_push_action(),
        &bot_caps(),
        &config_with(user),
        "fake-computer",
    );
    assert_eq!(verdict.outcome, GateOutcome::Allow);
    assert_eq!(verdict.layer, GateLayer::User);
}

#[test]
fn test_a_session_rule_for_the_action_key_allows_it() {
    let action = GateAction::command("shell", "npm run build", "/workspace", Vec::new());
    let user = UserRules {
        session: vec![action.key()],
        ..UserRules::default()
    };
    let verdict = judge(&action, &bot_caps(), &config_with(user), "fake-computer");
    assert_eq!(verdict.outcome, GateOutcome::Allow);
    assert_eq!(verdict.layer, GateLayer::User);
}

#[test]
fn test_a_user_hook_rule_denies_the_action_it_names() {
    let action = GateAction::command("shell", "npm run build", "/workspace", Vec::new());
    let user = UserRules {
        hook: vec![RuleEntry {
            key: action.key(),
            opinion: Opinion::Deny,
        }],
        ..UserRules::default()
    };
    let verdict = judge(&action, &bot_caps(), &config_with(user), "fake-computer");
    assert_eq!(verdict.outcome, GateOutcome::Deny);
    assert_eq!(verdict.layer, GateLayer::User);
}

#[test]
fn test_an_ask_style_rule_asks_for_the_action_it_names() {
    let action = GateAction::command("shell", "npm run build", "/workspace", Vec::new());
    let user = UserRules {
        ask_style: vec![action.key()],
        ..UserRules::default()
    };
    let verdict = judge(&action, &bot_caps(), &config_with(user), "fake-computer");
    assert_eq!(verdict.outcome, GateOutcome::Ask);
    assert_eq!(verdict.layer, GateLayer::User);
}

#[test]
fn test_an_empty_config_cannot_be_silently_relaxed_by_the_caller() {
    let action = GateAction::command("shell", "git push origin main", "/workspace", Vec::new());
    let verdict = judge(&action, &bot_caps(), &GateConfig::plain(), "fake-computer");
    assert_eq!(verdict.outcome, GateOutcome::Ask);
}

fn direct_config() -> GateConfig {
    let mut config = GateConfig::plain();
    config.modes.direct_mode = true;
    config
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

fn write_file(path: &str) -> GateAction {
    GateAction {
        tool: "edit_file".to_owned(),
        subject: ActionSubject::WriteFile {
            path: PathView::resolved(path, path),
            content: "x".to_owned(),
        },
        extra_paths: Vec::new(),
        workdir: "/workspace".to_owned(),
    }
}

#[test]
fn test_direct_mode_still_asks_before_a_bot_publish() {
    let verdict = judge(&git_push_action(), &bot_caps(), &direct_config(), "bot-1");
    assert_eq!(verdict.outcome, GateOutcome::Ask);
}

#[test]
fn test_direct_mode_still_asks_before_a_delete_outside_the_workdir() {
    let action = GateAction::command(
        "shell",
        "rm /etc/hosts",
        "/workspace",
        vec![PathView::resolved("/etc/hosts", "/etc/hosts")],
    );
    let verdict = judge(&action, &bot_caps(), &direct_config(), "bot-1");
    assert_eq!(verdict.outcome, GateOutcome::Ask);
}

#[test]
fn test_direct_mode_still_asks_before_a_write_outside_the_workdir_on_the_user_machine() {
    let verdict = judge(
        &write_file("/Users/me/.zshrc"),
        &user_caps(),
        &direct_config(),
        "user-mac",
    );
    assert_eq!(verdict.outcome, GateOutcome::Ask);
}

#[test]
fn test_an_always_list_still_allows_a_six_class_under_direct_mode() {
    let user = UserRules {
        always: AlwaysList {
            entries: vec![AlwaysEntry {
                action: format!(
                    "command {} on bot-1",
                    crate::command_key::command_action_key("git push origin main"),
                ),
                added_at: TimestampMs::from_millis(1),
                from_bot: anybot_foundation::BotId::generate(&anybot_foundation::SystemClock),
            }],
        },
        ..UserRules::default()
    };
    let mut config = config_with(user);
    config.modes.direct_mode = true;
    let verdict = judge(&git_push_action(), &bot_caps(), &config, "bot-1");
    assert_eq!(verdict.outcome, GateOutcome::Allow);
    assert_eq!(verdict.layer, GateLayer::User);
}

#[test]
fn test_direct_mode_still_runs_a_delete_inside_the_bot_folder() {
    let action = GateAction::command(
        "shell",
        "rm notes.md",
        "/workspace",
        vec![PathView::resolved("notes.md", "/workspace/notes.md")],
    );
    let verdict = judge(&action, &bot_caps(), &direct_config(), "bot-1");
    assert_eq!(verdict.outcome, GateOutcome::Allow);
}
