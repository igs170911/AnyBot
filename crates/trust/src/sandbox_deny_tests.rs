use proptest::prelude::*;

use crate::sandbox_deny::{DENIED_COMMAND_KINDS, deny_token};

const DANGEROUS_LINES: [&str; 10] = [
    "sudo ls",
    "su root",
    "rm -rf /",
    "rm -rf ~/projects",
    "mkfs.ext4 /dev/sda1",
    "dd if=/dev/zero of=/x",
    "shutdown -h now",
    "reboot now",
    "diskutil eraseVolume APFS /dev/x",
    "chmod -R 777 /",
];

const WRAPPER_WORDS: [&str; 7] = [
    "env A=1",
    "command",
    "nohup",
    "time",
    "nice -n 0",
    "stdbuf",
    "timeout 5",
];

const JOIN_WORDS: [&str; 6] = ["; ", " && ", " || ", " | ", " & ", "\n"];

const HARMLESS_LINES: [&str; 5] = ["ls -la", "echo hi", "cat notes.txt", "git status", "pwd"];

const BENIGN_LINE: &str = "echo hi";

type Skeleton = (usize, Vec<usize>, Vec<usize>, Vec<usize>, bool);

fn skeleton_strategy() -> BoxedStrategy<Skeleton> {
    (
        0usize..DENIED_COMMAND_KINDS.len(),
        prop::collection::vec(0usize..WRAPPER_WORDS.len(), 0..3),
        prop::collection::vec(0usize..JOIN_WORDS.len(), 1..3),
        prop::collection::vec(0usize..HARMLESS_LINES.len(), 1..4),
        any::<bool>(),
    )
        .boxed()
}

fn joined(core: &str, joins: &[usize], harms: &[usize]) -> String {
    let mut text = String::new();
    let mut rest = harms;
    if let Some(first) = rest.first() {
        let token = JOIN_WORDS
            .get(joins.first().copied().unwrap_or(0))
            .unwrap_or(&" ");
        text = format!("{}{token}", HARMLESS_LINES.get(*first).unwrap_or(&"ls -la"));
        rest = rest.get(1..).unwrap_or(&[]);
    }
    text.push_str(core);
    for (join, harm) in joins.iter().skip(1).zip(rest.iter()) {
        let token = JOIN_WORDS.get(*join).unwrap_or(&" ");
        let harm_line = HARMLESS_LINES.get(*harm).unwrap_or(&"ls -la");
        text = format!("{text}{token}{harm_line}");
    }
    text
}

fn build_core(layers: &[usize], prefix: bool, inner: &str) -> String {
    let mut core = String::from(inner);
    for layer in layers.iter().rev() {
        let wrapper = WRAPPER_WORDS.get(*layer).unwrap_or(&"command");
        core = format!("{wrapper} {core}");
    }
    if prefix {
        core = format!("VAR=1 {core}");
    }
    core
}

#[test]
fn t_sandbox_deny_1() {
    for (index, kind) in DENIED_COMMAND_KINDS.iter().enumerate() {
        let kind = *kind;
        let line = DANGEROUS_LINES.get(index).unwrap_or(&"sudo ls");
        assert_eq!(deny_token(line, false), Some(kind), "line {line}");
        let upper = line.to_uppercase();
        assert_eq!(deny_token(&upper, false), Some(kind), "line {upper}");
        let spread = line.replace(' ', "   ");
        assert_eq!(deny_token(&spread, false), Some(kind), "line {spread}");
    }
    assert_eq!(
        crate::gate_flow::rule_code("sandbox-deny"),
        "gate-command-denied"
    );
}

#[test]
fn test_sandbox_deny_split_danger_lands_between_harmless() {
    for token in JOIN_WORDS {
        for dangerous in DANGEROUS_LINES {
            let line = format!("ls -la{token}{dangerous}{token}pwd");
            assert!(
                deny_token(&line, false).is_some(),
                "line {line} must be denied through every connector"
            );
            let benign = format!("ls -la{token}echo hi{token}pwd");
            assert!(
                deny_token(&benign, false).is_none(),
                "line {benign} must stay allowed"
            );
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(3000))]

    #[test]
    fn t_sandbox_deny_2(skeleton in skeleton_strategy()) {
        let (kind_index, layers, joins, harms, prefix) = skeleton;
        let kind = DENIED_COMMAND_KINDS
            .get(kind_index)
            .copied()
            .unwrap_or("sudo");
        let dangerous = DANGEROUS_LINES.get(kind_index).unwrap_or(&"sudo ls");
        let core = build_core(&layers, prefix, dangerous);
        let full = joined(&core, &joins, &harms);
        prop_assert_eq!(deny_token(&full, false), Some(kind), "line {}", full);

        let benign_core = build_core(&layers, prefix, BENIGN_LINE);
        let benign_full = joined(&benign_core, &joins, &harms);
        prop_assert_eq!(deny_token(&benign_full, false), None, "line {}", benign_full);
    }
}
