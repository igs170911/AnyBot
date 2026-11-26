use crate::action::GateAction;
use crate::evidence_payload::{payload_kind, payload_of};
use crate::runner::OpRejection;
use anybot_foundation::{Clock, EvidenceId, TaskId, TurnId};
use anybot_protocol::{
    ActionOutcome, ActualResult, EvidenceKind, EvidencePayload, EvidenceRecord, EvidenceShortId,
    GateRecord, JudgementSource, ReasonPayload,
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
pub(super) fn build_evidence(
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
pub(crate) fn rejected_evidence(
    context: &ActionContext,
    gate: &GateRecord,
    reason: ReasonPayload,
    clock: &dyn Clock,
) -> EvidenceRecord {
    base_record(
        context,
        gate,
        clock,
        EvidenceKind::Rejected,
        ActionOutcome::Rejected,
        gate.source,
        EvidencePayload::Rejected { reason },
    )
}

#[must_use]
pub(crate) fn failed_evidence(
    context: &ActionContext,
    gate: &GateRecord,
    reason: ReasonPayload,
    clock: &dyn Clock,
) -> EvidenceRecord {
    base_record(
        context,
        gate,
        clock,
        EvidenceKind::Failed,
        ActionOutcome::Failed,
        JudgementSource::ProgramCheck,
        EvidencePayload::Rejected { reason },
    )
}

#[must_use]
pub(crate) fn approved_evidence(
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
pub(crate) fn rejection_reason(rejection: &OpRejection) -> ReasonPayload {
    let code = match rejection.kind {
        crate::runner::OpRejectionKind::TicketMismatch => {
            crate::message_codes::CODE_GATE_TICKET_MISMATCH
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{ActionSubject, GateAction, PathView};
    use anybot_foundation::{ContentDigest, SystemClock};
    use anybot_protocol::{
        BeforeDigest, GateNo, GateOutcome, ReadOutcome, RuleLayer, TargetKind, WriteOutcome,
    };

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
    fn test_refused_and_failed_steps_have_their_own_kinds() {
        let gate = GateRecord::new(
            GateNo(4),
            RuleLayer::Default,
            "bot-machine-files".to_owned(),
            JudgementSource::Rule,
            GateOutcome::Deny,
            false,
        );
        let reason = ReasonPayload {
            code: "gate-denied".to_owned(),
            params: Vec::new(),
        };
        let refused = rejected_evidence(&context(), &gate, reason.clone(), &SystemClock);
        assert_eq!(refused.kind, EvidenceKind::Rejected);
        assert_eq!(refused.outcome, ActionOutcome::Rejected);
        let broken = failed_evidence(&context(), &gate, reason.clone(), &SystemClock);
        assert_eq!(broken.kind, EvidenceKind::Failed);
        assert_eq!(broken.outcome, ActionOutcome::Failed);
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
