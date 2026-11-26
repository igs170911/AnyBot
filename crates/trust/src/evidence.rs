use crate::action::GateAction;
use crate::evidence_payload::{payload_kind, payload_of};
use crate::runner::OpRejection;
use anybot_foundation::{Clock, EvidenceId, TaskId, TurnId};
use anybot_protocol::{
    ActionOutcome, ActualResult, EvidenceKind, EvidencePayload, EvidenceRecord, EvidenceShortId,
    GateOutcome, GateRecord, JudgementSource, NotExecutedReason, ReasonPayload,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionContext {
    pub task: TaskId,
    pub turn: TurnId,
    pub evidence_no: u32,
    pub computer: String,
    pub workdir: String,
    pub tool: String,
    pub classes: Vec<String>,
}

impl ActionContext {
    #[must_use]
    pub fn action_views(&self, action: &GateAction) -> Vec<String> {
        action
            .path_views()
            .iter()
            .map(|p| p.effective(&self.workdir))
            .collect()
    }
}

#[must_use]
pub fn build_evidence(
    context: &ActionContext,
    gate: &GateRecord,
    result: &ActualResult,
    clock: &dyn Clock,
) -> EvidenceRecord {
    let kind = payload_kind(result);
    let payload = payload_of(result);
    EvidenceRecord {
        id: EvidenceId::generate(clock),
        short: EvidenceShortId(context.evidence_no),
        at: clock.now_ms(),
        task: context.task,
        turn: context.turn,
        computer: context.computer.clone(),
        workdir: context.workdir.clone(),
        tool: context.tool.clone(),
        classes: context.classes.clone(),
        gate: gate.clone(),
        kind,
        outcome: ActionOutcome::Completed,
        payload,
        verification: None,
        source: JudgementSource::Rule,
        ungated: false,
    }
}

#[must_use]
pub fn rejected_evidence(
    context: &ActionContext,
    gate: &GateRecord,
    reason: ReasonPayload,
    clock: &dyn Clock,
) -> EvidenceRecord {
    base_record(
        context,
        gate,
        clock,
        EvidenceKind::Approved,
        ActionOutcome::Rejected,
        gate.source,
        EvidencePayload::Rejected { reason },
    )
}

#[must_use]
pub fn failed_evidence(
    context: &ActionContext,
    gate: &GateRecord,
    reason: ReasonPayload,
    clock: &dyn Clock,
) -> EvidenceRecord {
    base_record(
        context,
        gate,
        clock,
        EvidenceKind::Approved,
        ActionOutcome::Failed,
        JudgementSource::ProgramCheck,
        EvidencePayload::Rejected { reason },
    )
}

#[must_use]
pub fn approved_evidence(
    context: &ActionContext,
    gate: &GateRecord,
    source: JudgementSource,
    clock: &dyn Clock,
) -> EvidenceRecord {
    base_record(
        context,
        gate,
        clock,
        EvidenceKind::Approved,
        ActionOutcome::Completed,
        source,
        EvidencePayload::Approved { gate: gate.clone() },
    )
}

#[must_use]
pub fn not_executed_evidence(
    context: &ActionContext,
    gate: &GateRecord,
    reason: NotExecutedReason,
    detail: ReasonPayload,
    clock: &dyn Clock,
) -> EvidenceRecord {
    base_record(
        context,
        gate,
        clock,
        EvidenceKind::Approved,
        ActionOutcome::NotExecuted,
        JudgementSource::Rule,
        EvidencePayload::NotExecuted { reason, detail },
    )
}

#[must_use]
pub fn ungated_evidence(
    context: &ActionContext,
    gate: &GateRecord,
    result: &ActualResult,
    clock: &dyn Clock,
) -> EvidenceRecord {
    let mut record = build_evidence(context, gate, result, clock);
    record.ungated = true;
    record.source = JudgementSource::Ungated;
    record
}

#[must_use]
pub fn rejection_reason(rejection: &OpRejection) -> ReasonPayload {
    let code = match rejection.kind {
        crate::runner::OpRejectionKind::PassNotAuthorized => {
            crate::message_codes::CODE_GATE_PASS_NOT_AUTHORIZED
        }
        crate::runner::OpRejectionKind::Unreachable => {
            crate::message_codes::CODE_TOOL_COMPUTER_UNREACHABLE
        }
        crate::runner::OpRejectionKind::Operation => {
            crate::message_codes::CODE_TOOL_OPERATION_FAILED
        }
    };
    ReasonPayload {
        code: code.to_owned(),
        params: vec![("reason".to_owned(), rejection.reason.clone())],
    }
}

fn base_record(
    context: &ActionContext,
    gate: &GateRecord,
    clock: &dyn Clock,
    kind: EvidenceKind,
    outcome: ActionOutcome,
    source: JudgementSource,
    payload: EvidencePayload,
) -> EvidenceRecord {
    EvidenceRecord {
        id: EvidenceId::generate(clock),
        short: EvidenceShortId(context.evidence_no),
        at: clock.now_ms(),
        task: context.task,
        turn: context.turn,
        computer: context.computer.clone(),
        workdir: context.workdir.clone(),
        tool: context.tool.clone(),
        classes: context.classes.clone(),
        gate: gate.clone(),
        kind,
        outcome,
        payload,
        verification: None,
        source,
        ungated: false,
    }
}

#[must_use]
pub fn gate_allowed(gate: &GateRecord) -> bool {
    gate.outcome == GateOutcome::Allow
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{ActionSubject, GateAction, PathView};
    use anybot_foundation::{ContentDigest, SystemClock};
    use anybot_protocol::{BeforeDigest, GateNo, ReadOutcome, RuleLayer, TargetKind, WriteOutcome};

    fn context() -> ActionContext {
        ActionContext {
            task: TaskId::generate(&SystemClock),
            turn: TurnId::generate(&SystemClock),
            evidence_no: 3,
            computer: "bot-1".to_owned(),
            workdir: "/workspace".to_owned(),
            tool: "read_file".to_owned(),
            classes: Vec::new(),
        }
    }

    #[test]
    fn test_evidence_from_read_result_covers_the_seen_fields() {
        let gate = GateRecord::new(
            GateNo(4),
            RuleLayer::Default,
            "bot-machine-files".to_owned(),
            JudgementSource::Rule,
            GateOutcome::Allow,
            true,
        );
        let result = ActualResult::Read(ReadOutcome {
            path: "/workspace/notes.md".to_owned(),
            kind: TargetKind::File,
            digest: ContentDigest::sha256(b"abc"),
            size: 3,
            content: "abc".to_owned(),
        });
        let record = build_evidence(&context(), &gate, &result, &SystemClock);
        assert_eq!(record.short.number(), 3);
        assert_eq!(record.kind, EvidenceKind::Seen);
        assert_eq!(record.gate.rule, "bot-machine-files");
    }

    #[test]
    fn test_evidence_write_result_records_before_and_after_digests() {
        let gate = GateRecord::new(
            GateNo(4),
            RuleLayer::Default,
            "bot-machine-files".to_owned(),
            JudgementSource::Rule,
            GateOutcome::Allow,
            false,
        );
        let result = ActualResult::Write(WriteOutcome {
            path: "/workspace/notes.md".to_owned(),
            before: BeforeDigest::Present(ContentDigest::sha256(b"old")),
            after: ContentDigest::sha256(b"new"),
            size: 3,
        });
        let record = build_evidence(&context(), &gate, &result, &SystemClock);
        assert_eq!(record.kind, EvidenceKind::Changed);
        assert_eq!(record.outcome, ActionOutcome::Completed);
    }

    #[test]
    fn test_evidence_ungated_report_marks_the_source_and_flag() {
        let gate = GateRecord::new(
            GateNo(4),
            RuleLayer::Default,
            "unverified-report".to_owned(),
            JudgementSource::Ungated,
            GateOutcome::Allow,
            false,
        );
        let result = ActualResult::Read(ReadOutcome {
            path: "/workspace/x".to_owned(),
            kind: TargetKind::File,
            digest: ContentDigest::sha256(b"x"),
            size: 1,
            content: "x".to_owned(),
        });
        let record = ungated_evidence(&context(), &gate, &result, &SystemClock);
        assert!(record.ungated);
        assert_eq!(record.source, JudgementSource::Ungated);
        assert_eq!(record.gate.outcome, GateOutcome::Allow);
    }

    #[test]
    fn test_evidence_context_maps_relative_targets_to_the_workdir() {
        let action = GateAction {
            tool: "read_file".to_owned(),
            subject: ActionSubject::ReadFile {
                path: PathView::raw("notes.md"),
            },
            extra_paths: Vec::new(),
            workdir: "/workspace".to_owned(),
        };
        let views = context().action_views(&action);
        assert_eq!(views, vec!["/workspace/notes.md".to_owned()]);
    }
}
