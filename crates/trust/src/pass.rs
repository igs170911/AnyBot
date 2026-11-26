use anybot_foundation::TimestampMs;
use anybot_protocol::GateRecord;

use crate::action::ActionKey;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pass {
    key: ActionKey,
    gate: GateRecord,
    issued_at: TimestampMs,
    companion_subjects: Vec<String>,
}

impl Pass {
    pub(crate) fn issue(key: ActionKey, gate: GateRecord, issued_at: TimestampMs) -> Self {
        let companion_subjects = companion_of(&key);
        Pass {
            key,
            gate,
            issued_at,
            companion_subjects,
        }
    }

    #[must_use]
    pub fn gate(&self) -> &GateRecord {
        &self.gate
    }

    #[must_use]
    pub fn issued_at(&self) -> TimestampMs {
        self.issued_at
    }

    #[must_use]
    pub fn authorizes(&self, key: &ActionKey) -> bool {
        if &self.key == key {
            return true;
        }
        self.matches_companion(key)
    }

    fn matches_companion(&self, key: &ActionKey) -> bool {
        let (tool, subject) = key.parts();
        let (own_tool, _) = self.key.parts();
        tool == own_tool && self.companion_subjects.iter().any(|s| s == subject)
    }

    #[must_use]
    pub fn tool(&self) -> &str {
        self.key.parts().0
    }
}

fn companion_of(key: &ActionKey) -> Vec<String> {
    let subject = key.parts().1;
    if let Some(path) = subject.strip_prefix("write:") {
        return vec![format!("read:{path}")];
    }
    if let Some(path) = subject.strip_prefix("edit:") {
        return vec![format!("read:{path}"), format!("write:{path}")];
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use anybot_protocol::{GateNo, GateOutcome, JudgementSource, RuleLayer};

    fn allow_gate() -> GateRecord {
        GateRecord::new(
            GateNo(1),
            RuleLayer::Default,
            "bot-machine-command".to_owned(),
            JudgementSource::Rule,
            GateOutcome::Allow,
            false,
        )
    }

    #[test]
    fn test_pass_authorizes_only_the_exact_action_it_was_issued_for() {
        let pass = Pass::issue(
            ActionKey::command("shell", "ls -la"),
            allow_gate(),
            TimestampMs::from_millis(10),
        );
        assert!(pass.authorizes(&ActionKey::command("shell", "ls   -la")));
        assert!(!pass.authorizes(&ActionKey::command("shell", "ls -lb")));
        assert!(!pass.authorizes(&ActionKey::file("read_file", "read", "notes.md")));
        assert_eq!(pass.issued_at.millis(), 10);
        assert_eq!(pass.gate().rule, "bot-machine-command");
        assert_eq!(pass.tool(), "shell");
    }

    #[test]
    fn test_pass_of_an_edit_authorizes_the_reread_and_the_write_of_the_same_file() {
        let pass = Pass::issue(
            ActionKey::file("edit_file", "edit", "notes.md"),
            allow_gate(),
            TimestampMs::from_millis(10),
        );
        assert!(pass.authorizes(&ActionKey::file("edit_file", "read", "notes.md")));
        assert!(pass.authorizes(&ActionKey::file("edit_file", "write", "notes.md")));
        assert!(!pass.authorizes(&ActionKey::file("edit_file", "read", "other.md")));
        assert!(!pass.authorizes(&ActionKey::file("read_file", "read", "notes.md")));
    }

    #[test]
    fn test_pass_of_a_write_authorizes_only_the_reread_of_the_same_file() {
        let pass = Pass::issue(
            ActionKey::file("edit_file", "write", "notes.md"),
            allow_gate(),
            TimestampMs::from_millis(10),
        );
        assert!(pass.authorizes(&ActionKey::file("edit_file", "read", "notes.md")));
        assert!(!pass.authorizes(&ActionKey::file("edit_file", "write", "other.md")));
    }
}
