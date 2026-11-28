use crate::action::{ActionKey, ActionSubject};
use anybot_foundation::TimestampMs;
use anybot_protocol::{CommandRequest, ComputerCaps, GateRecord};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpKind {
    Run,
    Read,
    ListDir,
    Search,
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
            ActionSubject::ListDir { .. } => OpKind::ListDir,
            ActionSubject::Search { .. } => OpKind::Search,
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
            ActionSubject::Command {
                line, timeout_ms, ..
            } => Some((line.as_str(), *timeout_ms)),
            ActionSubject::ReadFile { .. }
            | ActionSubject::ListDir { .. }
            | ActionSubject::Search { .. }
            | ActionSubject::WriteFile { .. }
            | ActionSubject::EditFile { .. }
            | ActionSubject::PathCheck { .. }
            | ActionSubject::Capabilities => None,
        }
    }

    #[must_use]
    pub fn command_request(&self) -> Option<CommandRequest> {
        match &self.subject {
            ActionSubject::Command {
                line,
                timeout_ms,
                env,
            } => Some(CommandRequest {
                command: line.clone(),
                timeout_ms: *timeout_ms,
                env: env.clone(),
            }),
            ActionSubject::ReadFile { .. }
            | ActionSubject::ListDir { .. }
            | ActionSubject::Search { .. }
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
            | ActionSubject::ListDir { path }
            | ActionSubject::Search { path, .. }
            | ActionSubject::WriteFile { path, .. }
            | ActionSubject::EditFile { path, .. }
            | ActionSubject::PathCheck { path } => path.resolved_text().or(Some(path.raw_text())),
            ActionSubject::Command { .. } | ActionSubject::Capabilities => None,
        }
    }

    #[must_use]
    pub fn search_query(&self) -> Option<(&str, bool)> {
        match &self.subject {
            ActionSubject::Search {
                pattern,
                files_only,
                ..
            } => Some((pattern.as_str(), *files_only)),
            ActionSubject::Command { .. }
            | ActionSubject::ReadFile { .. }
            | ActionSubject::ListDir { .. }
            | ActionSubject::WriteFile { .. }
            | ActionSubject::EditFile { .. }
            | ActionSubject::PathCheck { .. }
            | ActionSubject::Capabilities => None,
        }
    }

    #[must_use]
    pub fn content(&self) -> Option<&str> {
        match &self.subject {
            ActionSubject::WriteFile { content, .. } => Some(content.as_str()),
            ActionSubject::Command { .. }
            | ActionSubject::ReadFile { .. }
            | ActionSubject::ListDir { .. }
            | ActionSubject::Search { .. }
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
            | ActionSubject::ListDir { .. }
            | ActionSubject::Search { .. }
            | ActionSubject::WriteFile { .. }
            | ActionSubject::PathCheck { .. }
            | ActionSubject::Capabilities => None,
        }
    }
}
