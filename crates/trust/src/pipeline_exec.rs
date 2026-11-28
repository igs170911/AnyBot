use crate::evidence::ActionContext;
use crate::evidence::{failed_evidence, rejection_reason};
use crate::op_ticket::OpTicket;
use crate::pipeline::ActionRequest;
use crate::pipeline_run::{ActionRun, ModelResultView, OutcomeText, Summary};
use crate::pipeline_verify::{perform, plan_of, verified_evidence, verify};
use crate::runner::{GatedComputer, OpRejection, OpRejectionKind};
use anybot_foundation::Clock;
use anybot_protocol::{ActionOutcome, EvidenceRecord, GateRecord, NotExecutedReason};

pub(crate) fn executed_run(
    request: &ActionRequest,
    ticket: OpTicket,
    computer: &dyn GatedComputer,
    view: &dyn ModelResultView,
    clock: &dyn Clock,
) -> ActionRun {
    let gate = ticket.gate().clone();
    let plan = plan_of(&ticket);
    let op = perform(ticket, computer);
    match op {
        Ok(result) => {
            let verification = verify(&plan, &result, computer, clock);
            let evidence = verified_evidence(&request.context, &gate, &result, verification, clock);
            let model = view.result_text(&request.context.tool, &result);
            finish(gate, vec![evidence], model, &request.context)
        }
        Err(rejection) => failed_run(request, gate, &rejection, view, clock),
    }
}

fn failed_run(
    request: &ActionRequest,
    gate: GateRecord,
    rejection: &OpRejection,
    view: &dyn ModelResultView,
    clock: &dyn Clock,
) -> ActionRun {
    if rejection.kind == OpRejectionKind::Unreachable {
        return unreachable_run(request, &gate, rejection, view);
    }
    let reason = rejection_reason(rejection);
    let evidence = failed_evidence(&request.context, &gate, reason.clone(), clock);
    let model = view.failure_text(rejection);
    finish(gate, vec![evidence], model, &request.context)
}

fn unreachable_run(
    request: &ActionRequest,
    gate: &GateRecord,
    rejection: &OpRejection,
    view: &dyn ModelResultView,
) -> ActionRun {
    let summary = Summary {
        status: OutcomeText::of(
            crate::message_codes::CODE_TURN_NOT_EXECUTED,
            vec![
                (
                    "reason".to_owned(),
                    not_executed_word(NotExecutedReason::ComputerUnreachable).to_owned(),
                ),
                ("detail".to_owned(), rejection.reason.clone()),
            ],
        ),
        action: action_label(&request.subject),
        location: request.context.computer.clone(),
        result: OutcomeText::of(crate::message_codes::CODE_TOOL_EMPTY, Vec::new()),
        verification: None,
        evidence_numbers: Vec::new(),
    };
    ActionRun {
        gate: gate.clone(),
        evidence: Vec::new(),
        model: view.failure_text(rejection),
        summary,
    }
}

#[must_use]
pub(super) fn finish(
    gate: GateRecord,
    evidence: Vec<EvidenceRecord>,
    model: OutcomeText,
    context: &ActionContext,
) -> ActionRun {
    let numbers = evidence.iter().map(|e| e.short.number()).collect();
    let summary = Summary {
        status: OutcomeText::of(status_token(evidence.last()), Vec::new()),
        action: context.tool.clone(),
        location: context.computer.clone(),
        result: model.clone(),
        verification: None,
        evidence_numbers: numbers,
    };
    ActionRun {
        gate,
        evidence,
        model,
        summary,
    }
}

fn status_token(evidence: Option<&EvidenceRecord>) -> &'static str {
    match evidence.map(|e| e.outcome) {
        Some(ActionOutcome::Completed) => crate::message_codes::CODE_TURN_COMPLETED,
        Some(ActionOutcome::Failed) => crate::message_codes::CODE_TURN_TOOL_FAILED,
        Some(ActionOutcome::Rejected) => crate::message_codes::CODE_TURN_REJECTED,
        Some(ActionOutcome::NotExecuted) | None => crate::message_codes::CODE_TURN_NOT_EXECUTED,
    }
}

fn not_executed_word(reason: NotExecutedReason) -> &'static str {
    match reason {
        NotExecutedReason::ComputerUnreachable => "computer-unreachable",
        NotExecutedReason::PriorActionRejected => "prior-action-rejected",
        NotExecutedReason::Stopped => "stopped",
        NotExecutedReason::ApprovalExpired => "approval-expired",
    }
}

pub(crate) fn action_label(subject: &crate::pipeline::RequestedAction) -> String {
    match subject {
        crate::pipeline::RequestedAction::Command { line, .. } => line.clone(),
        crate::pipeline::RequestedAction::ReadFile { path }
        | crate::pipeline::RequestedAction::ListDir { path }
        | crate::pipeline::RequestedAction::Search { path, .. }
        | crate::pipeline::RequestedAction::WriteFile { path, .. }
        | crate::pipeline::RequestedAction::EditFile { path, .. }
        | crate::pipeline::RequestedAction::PathCheck { path } => path.clone(),
        crate::pipeline::RequestedAction::Capabilities => "capabilities".to_owned(),
    }
}
