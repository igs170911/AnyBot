use proptest::prelude::*;

use crate::action::GateAction;
use crate::command_read::{ReadLine, read_line};
use crate::gate_engine::GateLayer;
use crate::gate_flow::GateConfig;
use crate::gate_full::ModeFlags;
use crate::sandbox_deny::{DENIED_COMMAND_KINDS, deny_token};
use anybot_protocol::{CapState, ComputerCaps, ComputerOwner, ComputerPlatform, GateOutcome};

const OPAQUE_LINES: [&str; 18] = [
    "( reboot )",
    "{ reboot; }",
    "! reboot",
    "\\reboot",
    "echo `reboot`",
    "echo $(reboot)",
    "echo ${HOME}",
    "diff <(ls) <(ls)",
    "tee >(cat)",
    "cat << EOF",
    "eval reboot",
    "source ./setup.sh",
    ". ./setup.sh",
    "sh -c reboot",
    "bash -lc reboot",
    "echo hi | sh",
    "alias reboot=ls",
    "git -c alias.st=status st",
];

const MORE_OPAQUE: [&str; 4] = ["cd", "cd -", "pushd /tmp", "popd"];

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

fn direct_config() -> GateConfig {
    let mut config = GateConfig::plain();
    config.modes = ModeFlags {
        ask_every_time: false,
        yolo: true,
        direct_mode: true,
        mdm_locks_direct: false,
    };
    config
}

fn judge_line(line: &str) -> GateOutcome {
    let action = GateAction::command("shell", line, "/workspace", Vec::new());
    crate::gate_flow::judge(&action, &bot_caps(), &direct_config(), "bot-1").outcome
}

#[test]
fn test_direct_mode_asks_for_each_unrecognized_command_shape() {
    for line in OPAQUE_LINES.iter().chain(MORE_OPAQUE.iter()) {
        assert_eq!(judge_line(line), GateOutcome::Ask, "line {line}");
        assert!(
            matches!(read_line(line), ReadLine::Opaque(_)),
            "line {line}"
        );
    }
}

#[test]
fn test_direct_mode_still_denies_a_recognized_blocked_command() {
    assert_eq!(
        judge_line("sudo nohup mkfs.ext4 /dev/sda1"),
        GateOutcome::Deny
    );
    assert_eq!(judge_line("rm -fr /"), GateOutcome::Deny);
    assert_eq!(judge_line("rm -r -f /"), GateOutcome::Deny);
    assert_eq!(judge_line("ls -la"), GateOutcome::Allow);
}

#[test]
fn test_direct_mode_keeps_an_unreadable_command_on_the_floor() {
    let action = GateAction::command("shell", "( reboot )", "/workspace", Vec::new());
    let verdict = crate::gate_flow::judge(&action, &bot_caps(), &direct_config(), "bot-1");
    assert_eq!(verdict.outcome, GateOutcome::Ask);
    assert_eq!(verdict.layer, GateLayer::Floor);
    assert_eq!(verdict.rule, "tool-unreadable-shape");
}

#[test]
fn test_an_unknown_wrapper_flag_asks_instead_of_reading_the_inner_command() {
    assert_eq!(judge_line("xargs -d newline reboot"), GateOutcome::Ask);
    assert_eq!(judge_line("nice -10 reboot"), GateOutcome::Ask);
    assert_eq!(judge_line("sudo -s reboot"), GateOutcome::Ask);
    assert_eq!(deny_token("xargs -d newline reboot", true), None);
    assert_eq!(deny_token("nice -10 reboot", true), None);
    assert_eq!(deny_token("sudo -s reboot", true), None);
}

const WRAPPERS: [&str; 8] = [
    "env -i",
    "command -p",
    "nohup",
    "nice -n 1",
    "stdbuf -o L",
    "timeout --foreground 5",
    "time -p",
    "xargs -n 1",
];

const JOINS: [&str; 6] = ["; ", " && ", " || ", " | ", " & ", "\n"];

const HARMLESS: [&str; 4] = ["ls -la", "echo hi", "cat notes.txt", "pwd"];

const DANGER: [&str; 8] = [
    "su root",
    "rm -rf /",
    "mkfs.ext4 /dev/sda1",
    "dd if=/dev/zero of=/x",
    "shutdown -h now",
    "reboot",
    "diskutil eraseVolume APFS /dev/x",
    "chmod -R 777 /",
];

const KINDS: [&str; 8] = [
    "su",
    "rm -rf /",
    "mkfs",
    "dd",
    "shutdown",
    "reboot",
    "diskutil erase",
    "chmod -R 777 /",
];

fn wrap(layers: &[usize], inner: &str) -> String {
    let mut text = inner.to_owned();
    for layer in layers {
        let wrapper = WRAPPERS.get(*layer).copied().unwrap_or("command -p");
        text = format!("{wrapper} {text}");
    }
    text
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    #[test]
    fn test_recognized_shapes_deny_only_the_blocked_command(
        kind in 0usize..DANGER.len(),
        layers in prop::collection::vec(0usize..WRAPPERS.len(), 0..3),
        joins in prop::collection::vec(0usize..JOINS.len(), 1..3),
        harms in prop::collection::vec(0usize..HARMLESS.len(), 1..3),
        prefix in any::<bool>(),
    ) {
        let inner = DANGER.get(kind).copied().unwrap_or("reboot");
        let expected = KINDS.get(kind).copied().unwrap_or("reboot");
        let mut core = wrap(&layers, inner);
        if prefix {
            core = format!("FOO=1 {core}");
        }
        let mut text = format!(
            "{}{}",
            HARMLESS.first().copied().unwrap_or("ls -la"),
            JOINS.first().copied().unwrap_or("; ")
        );
        text.push_str(&core);
        for (join, harm) in joins.iter().zip(harms.iter()) {
            let token = JOINS.get(*join).copied().unwrap_or("; ");
            let harm_line = HARMLESS.get(*harm).copied().unwrap_or("pwd");
            text = format!("{text}{token}{harm_line}");
        }
        prop_assert_eq!(deny_token(&text, true), Some(expected));
        prop_assert!(DENIED_COMMAND_KINDS.contains(&expected));
    }

    #[test]
    fn test_an_unrecognized_shape_never_runs_directly_under_direct_mode(
        tail in prop::sample::select(OPAQUE_LINES.as_slice()),
        prefix in prop::sample::select(HARMLESS.as_slice()),
        wrapper in prop::sample::select(WRAPPERS.as_slice()),
    ) {
        let line = format!("{wrapper} {prefix}; {tail}");
        let action = GateAction::command("shell", &line, "/workspace", Vec::new());
        let verdict = crate::gate_flow::judge(&action, &bot_caps(), &direct_config(), "bot-1");
        prop_assert_ne!(verdict.outcome, GateOutcome::Allow);
        prop_assert_eq!(verdict.layer, GateLayer::Floor);
    }
}
