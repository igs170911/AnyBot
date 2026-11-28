use crate::action::{ActionKey, ActionSubject};
use anybot_foundation::TimestampMs;
use anybot_protocol::{ComputerCaps, GateRecord};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpKind {
    Run,
    Read,
    Write,
    Edit,
    Path,
    Caps,
}

impl OpKind {
    #[must_use]
    pub fn of_subject(subject: &ActionSubject) -> Self {
        match subject {
            ActionSubject::Command { .. } => OpKind::Run,
            ActionSubject::ReadFile { .. } => OpKind::Read,
            ActionSubject::WriteFile { .. } => OpKind::Write,
            ActionSubject::EditFile { .. } => OpKind::Edit,
            ActionSubject::PathCheck { .. } => OpKind::Path,
            ActionSubject::Capabilities => OpKind::Caps,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct OpTicket {
    kind: OpKind,
    computer: String,
    caps: ComputerCaps,
    workdir: String,
    subject: ActionSubject,
    gate: GateRecord,
    key: ActionKey,
    issued_at: TimestampMs,
}

impl OpTicket {
    pub(crate) fn issue(
        subject: &ActionSubject,
        key: ActionKey,
        gate: GateRecord,
        computer: &str,
        caps: &ComputerCaps,
        workdir: &str,
        issued_at: TimestampMs,
    ) -> Self {
        OpTicket {
            kind: OpKind::of_subject(subject),
            computer: computer.to_owned(),
            caps: caps.clone(),
            workdir: workdir.to_owned(),
            subject: subject.clone(),
            gate,
            key,
            issued_at,
        }
    }

    #[must_use]
    pub fn kind(&self) -> OpKind {
        self.kind
    }

    #[must_use]
    pub fn computer(&self) -> &str {
        &self.computer
    }

    #[must_use]
    pub fn gate(&self) -> &GateRecord {
        &self.gate
    }

    #[must_use]
    pub fn key(&self) -> &ActionKey {
        &self.key
    }

    #[must_use]
    pub fn issued_at(&self) -> TimestampMs {
        self.issued_at
    }

    #[must_use]
    pub fn caps(&self) -> &ComputerCaps {
        &self.caps
    }

    #[must_use]
    pub fn workdir(&self) -> &str {
        &self.workdir
    }

    #[must_use]
    pub fn fits(&self, computer: &str, kind: OpKind) -> bool {
        self.computer == computer && self.kind == kind
    }

    #[must_use]
    pub fn matches_computer(&self, computer: &str, caps: &ComputerCaps, workdir: &str) -> bool {
        self.computer == computer && &self.caps == caps && self.workdir == workdir
    }

    #[must_use]
    pub fn command_line(&self) -> Option<(&str, u64)> {
        match &self.subject {
            ActionSubject::Command { line, timeout_ms } => Some((line.as_str(), *timeout_ms)),
            ActionSubject::ReadFile { .. }
            | ActionSubject::WriteFile { .. }
            | ActionSubject::EditFile { .. }
            | ActionSubject::PathCheck { .. }
            | ActionSubject::Capabilities => None,
        }
    }

    #[must_use]
    pub fn path(&self) -> Option<&str> {
        self.bound_path()
    }

    #[must_use]
    pub fn bound_path(&self) -> Option<&str> {
        match &self.subject {
            ActionSubject::ReadFile { path }
            | ActionSubject::WriteFile { path, .. }
            | ActionSubject::EditFile { path, .. }
            | ActionSubject::PathCheck { path } => path.resolved_text().or(Some(path.raw_text())),
            ActionSubject::Command { .. } | ActionSubject::Capabilities => None,
        }
    }

    #[must_use]
    pub fn content(&self) -> Option<&str> {
        match &self.subject {
            ActionSubject::WriteFile { content, .. } => Some(content.as_str()),
            ActionSubject::Command { .. }
            | ActionSubject::ReadFile { .. }
            | ActionSubject::EditFile { .. }
            | ActionSubject::PathCheck { .. }
            | ActionSubject::Capabilities => None,
        }
    }

    #[must_use]
    pub fn texts(&self) -> Option<(&str, &str)> {
        match &self.subject {
            ActionSubject::EditFile {
                old_text, new_text, ..
            } => Some((old_text.as_str(), new_text.as_str())),
            ActionSubject::Command { .. }
            | ActionSubject::ReadFile { .. }
            | ActionSubject::WriteFile { .. }
            | ActionSubject::PathCheck { .. }
            | ActionSubject::Capabilities => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::PathView;
    use anybot_protocol::{
        ComputerCaps, ComputerOwner, ComputerPlatform, GateOutcome, GateStep, JudgementSource,
        RuleLayer,
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
}
