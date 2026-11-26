use anybot_foundation::ContentDigest;
use anybot_foundation::{EvidenceId, TaskId, TimestampMs, TurnId};
use serde::{Deserialize, Serialize};

use crate::computer_results::BeforeDigest;
use crate::gate_record::{GateOutcome, GateRecord, JudgementSource};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    Seen,
    Changed,
    Executed,
    Approved,
    Rejected,
    Failed,
    NotExecuted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum ActionOutcome {
    Completed,
    Failed,
    Rejected,
    NotExecuted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum NotExecutedReason {
    ComputerUnreachable,
    PriorActionRejected,
    Stopped,
    ApprovalExpired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum VerificationResult {
    Passed,
    Failed,
    NothingToCheck,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct VerificationRecord {
    pub result: VerificationResult,
    pub source: JudgementSource,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct ReasonPayload {
    pub code: String,
    pub params: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum EvidencePayload {
    Seen {
        path: String,
        #[cfg_attr(feature = "schema", schema(value_type = String))]
        digest: ContentDigest,
        size: u64,
    },
    Changed {
        path: String,
        before: BeforeDigest,
        #[cfg_attr(feature = "schema", schema(value_type = String))]
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
        #[cfg_attr(feature = "schema", schema(value_type = String))]
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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct EvidenceShortId(pub u32);

impl EvidenceShortId {
    #[must_use]
    pub fn number(&self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct EvidenceRecord {
    #[cfg_attr(feature = "schema", schema(value_type = String, format = "uuid"))]
    pub id: EvidenceId,
    pub short: EvidenceShortId,
    #[cfg_attr(feature = "schema", schema(value_type = u64))]
    pub at: TimestampMs,
    #[cfg_attr(feature = "schema", schema(value_type = String, format = "uuid"))]
    pub task: TaskId,
    #[cfg_attr(feature = "schema", schema(value_type = String, format = "uuid"))]
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

    #[must_use]
    pub fn display(&self) -> String {
        let outcome = format!("{:?}", self.outcome).to_lowercase();
        let detail = match &self.payload {
            EvidencePayload::Seen { path, digest, .. } => {
                format!("seen {path} sha256:{digest}")
            }
            EvidencePayload::Changed {
                path,
                before,
                after,
                ..
            } => {
                let prior = match before {
                    BeforeDigest::Absent => "absent".to_owned(),
                    BeforeDigest::Present(value) => format!("sha256:{value}"),
                };
                format!("changed {path} before {prior} after sha256:{after}")
            }
            EvidencePayload::Executed {
                command, exit_code, ..
            } => {
                let code = exit_code.map_or("none".to_owned(), |c| c.to_string());
                format!("ran {command} exit {code}")
            }
            EvidencePayload::Approved { .. } => "approved".to_owned(),
            EvidencePayload::Rejected { reason } => {
                format!("rejected {code}", code = reason.code)
            }
            EvidencePayload::NotExecuted { reason, .. } => {
                format!("not-executed {reason:?}").to_lowercase()
            }
        };
        format!(
            "E{} {} {} {}",
            self.short.number(),
            self.tool,
            outcome,
            detail
        )
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
