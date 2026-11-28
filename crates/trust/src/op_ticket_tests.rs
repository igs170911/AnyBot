use crate::action::{ActionKey, ActionSubject, PathView};
use crate::op_ticket::{OpKind, OpTicket};
use anybot_foundation::TimestampMs;
use anybot_protocol::{
    ComputerCaps, ComputerOwner, ComputerPlatform, GateOutcome, GateRecord, GateStep,
    JudgementSource, RuleLayer,
};

fn issued_caps() -> ComputerCaps {
    ComputerCaps::declared(ComputerPlatform::Linux, ComputerOwner::Bot)
}

fn allow_gate() -> GateRecord {
    GateRecord::new(
        GateStep::DefaultRule,
        RuleLayer::Default,
        "bot-machine-command".to_owned(),
        JudgementSource::Rule,
        GateOutcome::Allow,
        false,
    )
}

fn command_ticket(line: &str) -> OpTicket {
    let subject = ActionSubject::Command {
        line: line.to_owned(),
        timeout_ms: 1000,
        env: crate::sandbox_env::command_env_plan(),
    };
    let key = ActionKey::command("shell", line);
    OpTicket::issue(
        &subject,
        key,
        allow_gate(),
        "fake-computer",
        &issued_caps(),
        "/workspace",
        TimestampMs::from_millis(10),
    )
}

#[test]
fn test_a_ticket_carries_the_command_exactly_as_it_was_written() {
    let ticket = command_ticket("echo OK\nreboot");
    let (line, timeout) = ticket.command_line().unwrap();
    assert_eq!(line, "echo OK\nreboot");
    assert_eq!(timeout, 1000);
    let req = ticket.command_request().unwrap();
    assert_eq!(req.command, "echo OK\nreboot");
    assert_eq!(req.timeout_ms, 1000);
    assert_eq!(req.env, crate::sandbox_env::command_env_plan());
    assert_eq!(ticket.kind(), OpKind::Run);
    assert_eq!(ticket.computer(), "fake-computer");
}

#[test]
fn test_a_ticket_only_fits_the_computer_and_operation_it_names() {
    let ticket = command_ticket("ls -la");
    assert!(ticket.fits("fake-computer", OpKind::Run));
    assert!(!ticket.fits("other-computer", OpKind::Run));
    assert!(!ticket.fits("fake-computer", OpKind::Write));
}

#[test]
fn test_a_write_ticket_carries_the_content_and_the_edit_ticket_carries_both_texts() {
    let write_subject = ActionSubject::WriteFile {
        path: PathView::raw("notes.md"),
        content: "Keep 100% CASE".to_owned(),
    };
    let write_key = ActionKey::file("write_file", "write", "notes.md");
    let write = OpTicket::issue(
        &write_subject,
        write_key,
        allow_gate(),
        "fake-computer",
        &issued_caps(),
        "/workspace",
        TimestampMs::from_millis(10),
    );
    assert_eq!(write.content(), Some("Keep 100% CASE"));
    assert_eq!(write.path(), Some("notes.md"));
    let edit_subject = ActionSubject::EditFile {
        path: PathView::raw("notes.md"),
        old_text: "one".to_owned(),
        new_text: "Two".to_owned(),
    };
    let edit_key = ActionKey::file("edit_file", "edit", "notes.md");
    let edit = OpTicket::issue(
        &edit_subject,
        edit_key,
        allow_gate(),
        "fake-computer",
        &issued_caps(),
        "/workspace",
        TimestampMs::from_millis(10),
    );
    assert_eq!(edit.texts(), Some(("one", "Two")));
    assert_eq!(edit.kind(), OpKind::Edit);
}
