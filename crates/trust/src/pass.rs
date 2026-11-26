use anybot_foundation::TimestampMs;
use anybot_protocol::GateRecord;

use crate::action::ActionKey;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pass {
    key: ActionKey,
    gate: GateRecord,
    issued_at: TimestampMs,
}

impl Pass {
    pub(crate) fn issue(key: ActionKey, gate: GateRecord, issued_at: TimestampMs) -> Self {
        Pass {
            key,
            gate,
            issued_at,
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
        &self.key == key
    }

    #[must_use]
    pub fn tool(&self) -> &str {
        self.key.parts().0
    }
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
}
