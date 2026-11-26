use crate::action::ActionSubject;
use crate::evidence::{
    ActionContext, approved_evidence, failed_evidence, rejected_evidence, rejection_reason,
};
use crate::gate_flow::{GateVerdict, rule_code};
use crate::pass::Pass;
use crate::pipeline::{ActionRequest, AskTicket, GateAnswer};
use crate::pipeline_verify::{perform, verified_evidence, verify};
use crate::runner::{GatedComputer, OpRejection, OpRejectionKind};
use anybot_foundation::Clock;
use anybot_protocol::{
    ActionOutcome, ActualResult, EvidenceRecord, GateOutcome, GateRecord, JudgementSource,
    NotExecutedReason, ReasonPayload,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutcomeText {
    pub code: String,
    pub params: Vec<(String, String)>,
}

impl OutcomeText {
    #[must_use]
    pub fn of(code: &str, params: Vec<(String, String)>) -> Self {
        OutcomeText {
            code: code.to_owned(),
            params,
        }
    }

    #[must_use]
    pub fn from_payload(payload: &ReasonPayload) -> Self {
        OutcomeText {
            code: payload.code.clone(),
            params: payload.params.clone(),
        }
    }
}

pub trait ModelResultView: Send + Sync {
    fn result_text(&self, tool: &str, result: &ActualResult) -> OutcomeText;

    fn failure_text(&self, rejection: &OpRejection) -> OutcomeText;

    fn rejection_text(&self, gate: &GateRecord, params: &[(String, String)]) -> OutcomeText;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    pub status: OutcomeText,
    pub action: String,
    pub location: String,
    pub result: OutcomeText,
    pub verification: Option<OutcomeText>,
    pub evidence_numbers: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionRun {
    pub gate: GateRecord,
    pub evidence: Vec<EvidenceRecord>,
    pub model: OutcomeText,
    pub summary: Summary,
}

#[derive(Debug)]
pub enum RunStep {
    Executed(ActionRun),
    Ask(Box<AskTicket>),
    Denied(ActionRun),
}

impl RunStep {
    #[must_use]
    pub fn into_run(self) -> ActionRun {
        match self {
            RunStep::Executed(run) | RunStep::Denied(run) => run,
            RunStep::Ask(ticket) => ActionRun {
                gate: ticket.gate().clone(),
                evidence: Vec::new(),
                model: OutcomeText::of(crate::message_codes::CODE_GATE_ASK_PENDING, Vec::new()),
                summary: pending_summary(ticket.request()),
            },
        }
    }
}

fn pending_summary(request: &ActionRequest) -> Summary {
    Summary {
        status: OutcomeText::of(
            crate::message_codes::CODE_TURN_AWAITING_APPROVAL,
            Vec::new(),
        ),
        action: request.context.tool.clone(),
        location: request.context.computer.clone(),
        result: OutcomeText::of(crate::message_codes::CODE_TOOL_EMPTY, Vec::new()),
        verification: None,
        evidence_numbers: Vec::new(),
    }
}

#[must_use]
pub(crate) fn prepared_gate(
    gate: &GateRecord,
    answer: GateAnswer,
    source: JudgementSource,
) -> GateRecord {
    let mut record = gate.clone();
    record.source = source;
    record.outcome = match answer {
        GateAnswer::Allow => GateOutcome::Allow,
        GateAnswer::Deny => GateOutcome::Deny,
    };
    record
}

pub(crate) fn user_approved_run(
    request: &ActionRequest,
    gate: &GateRecord,
    pass: &Pass,
    computer: &dyn GatedComputer,
    view: &dyn ModelResultView,
    clock: &dyn Clock,
) -> ActionRun {
    let approved = approved_evidence(&request.context, gate, JudgementSource::You, clock);
    let mut run = executed_run(request, pass, computer, view, clock);
    run.gate = gate.clone();
    run.summary
        .evidence_numbers
        .insert(0, approved.short.number());
    run.evidence.insert(0, approved);
    run
}

pub(crate) fn user_rejected_run(
    request: &ActionRequest,
    gate: GateRecord,
    view: &dyn ModelResultView,
    clock: &dyn Clock,
) -> ActionRun {
    let params = vec![("action".to_owned(), action_label(&request.subject))];
    let reason = ReasonPayload {
        code: crate::message_codes::CODE_GATE_USER_REJECTED.to_owned(),
        params: params.clone(),
    };
    let evidence = rejected_evidence(&request.context, &gate, reason, clock);
    let model = view.rejection_text(&gate, &params);
    finish(gate, vec![evidence], model, &request.context)
}

pub(crate) fn denied_run(
    request: &ActionRequest,
    verdict: &GateVerdict,
    gate: &GateRecord,
    view: &dyn ModelResultView,
    clock: &dyn Clock,
) -> ActionRun {
    let params = gate_params(verdict, request);
    let reason = ReasonPayload {
        code: rule_code(&gate.rule),
        params: params.clone(),
    };
    let evidence = rejected_evidence(&request.context, gate, reason, clock);
    let model = view.rejection_text(gate, &params);
    finish(gate.clone(), vec![evidence], model, &request.context)
}

fn gate_params(verdict: &GateVerdict, request: &ActionRequest) -> Vec<(String, String)> {
    let mut params = verdict.params.clone();
    params.push(("computer".to_owned(), request.context.computer.clone()));
    params
}

pub fn executed_run(
    request: &ActionRequest,
    pass: &Pass,
    computer: &dyn GatedComputer,
    view: &dyn ModelResultView,
    clock: &dyn Clock,
) -> ActionRun {
    let gate = pass.gate().clone();
    let op = perform(pass, &request.subject, computer);
    match op {
        Ok(result) => {
            let verification = verify(pass, &request.subject, &result, computer);
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
pub fn finish(
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

fn action_label(subject: &ActionSubject) -> String {
    match subject {
        ActionSubject::Command { line, .. } => line.clone(),
        ActionSubject::ReadFile { path } => format!("read {}", path.raw),
        ActionSubject::WriteFile { path, .. } | ActionSubject::EditFile { path, .. } => {
            format!("write {}", path.raw)
        }
        ActionSubject::PathCheck { path } => format!("path {}", path.raw),
        ActionSubject::Capabilities => "capabilities".to_owned(),
    }
}
