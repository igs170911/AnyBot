use anybot_foundation::ContentDigest;
use anybot_foundation::{EvidenceId, TaskId, TimestampMs, TurnId};
use serde::{Deserialize, Serialize};

use crate::computer_results::BeforeDigest;
use crate::gate_record::{GateOutcome, GateRecord, JudgementSource};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    Seen,
    Changed,
    Executed,
    Approved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionOutcome {
    Completed,
    Failed,
    Rejected,
    NotExecuted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotExecutedReason {
    ComputerUnreachable,
    PriorActionRejected,
    Stopped,
    ApprovalExpired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationResult {
    Passed,
    Failed,
    NothingToCheck,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationRecord {
    pub result: VerificationResult,
    pub source: JudgementSource,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReasonPayload {
    pub code: String,
    pub params: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidencePayload {
    Seen {
        path: String,
        digest: ContentDigest,
        size: u64,
    },
    Changed {
        path: String,
        before: BeforeDigest,
        after: ContentDigest,
        size: u64,
    },
    Executed {
        command: String,
        computer: String,
        exit_code: Option<i32>,
        output_head: String,
        output_tail: String,
        output_omitted: u64,
        full_digest: ContentDigest,
        elapsed_ms: u64,
    },
    Approved {
        gate: GateRecord,
    },
    Rejected {
        reason: ReasonPayload,
    },
    NotExecuted {
        reason: NotExecutedReason,
        detail: ReasonPayload,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceShortId(pub u32);

impl EvidenceShortId {
    #[must_use]
    pub fn number(&self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceRecord {
    pub id: EvidenceId,
    pub short: EvidenceShortId,
    pub at: TimestampMs,
    pub task: TaskId,
    pub turn: TurnId,
    pub computer: String,
    pub workdir: String,
    pub tool: String,
    pub classes: Vec<String>,
    pub gate: GateRecord,
    pub kind: EvidenceKind,
    pub outcome: ActionOutcome,
    pub payload: EvidencePayload,
    pub verification: Option<VerificationRecord>,
    pub source: JudgementSource,
    pub ungated: bool,
}

impl EvidenceRecord {
    #[must_use]
    pub fn gate_outcome(&self) -> GateOutcome {
        self.gate.outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gate_record::{GateNo, RuleLayer};
    use anybot_foundation::SystemClock;

    fn record(kind: EvidenceKind, outcome: ActionOutcome) -> EvidenceRecord {
        EvidenceRecord {
            id: EvidenceId::generate(&SystemClock),
            short: EvidenceShortId(1),
            at: TimestampMs::from_millis(0),
            task: TaskId::generate(&SystemClock),
            turn: TurnId::generate(&SystemClock),
            computer: "bot-1".to_owned(),
            workdir: "/workspace".to_owned(),
            tool: "shell".to_owned(),
            classes: Vec::new(),
            gate: GateRecord::new(
                GateNo(1),
                RuleLayer::Default,
                "bot-machine-direct".to_owned(),
                JudgementSource::Rule,
                GateOutcome::Allow,
                true,
            ),
            kind,
            outcome,
            payload: EvidencePayload::Approved {
                gate: GateRecord::new(
                    GateNo(1),
                    RuleLayer::Default,
                    "bot-machine-direct".to_owned(),
                    JudgementSource::Rule,
                    GateOutcome::Allow,
                    true,
                ),
            },
            verification: None,
            source: JudgementSource::Rule,
            ungated: false,
        }
    }

    #[test]
    fn test_evidence_record_carries_gate_fields_and_short_id() {
        let item = record(EvidenceKind::Executed, ActionOutcome::Completed);
        assert_eq!(item.short.number(), 1);
        assert_eq!(item.gate.rule, "bot-machine-direct");
        assert!(item.gate.read_only);
        assert_eq!(item.kind, EvidenceKind::Executed);
        assert_eq!(item.source, JudgementSource::Rule);
    }

    #[test]
    fn test_evidence_outcome_is_independent_of_gate_outcome() {
        let item = record(EvidenceKind::Seen, ActionOutcome::Completed);
        assert_eq!(item.gate_outcome(), GateOutcome::Allow);
        assert_eq!(item.outcome, ActionOutcome::Completed);
    }
}
