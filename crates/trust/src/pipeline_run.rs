use crate::action::ActionSubject;
use crate::evidence::{
    ActionContext, approved_evidence, build_evidence, rejected_evidence, rejection_reason,
};
use crate::gate_flow::{GateVerdict, rule_code};
use crate::pass::Pass;
use crate::pipeline::{ActionRequest, AskTicket, GateAnswer};
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunStep {
    Executed(ActionRun),
    Ask(AskTicket),
    Denied(ActionRun),
}

impl RunStep {
    #[must_use]
    pub fn into_run(self) -> ActionRun {
        match self {
            RunStep::Executed(run) | RunStep::Denied(run) => run,
            RunStep::Ask(ticket) => ActionRun {
                gate: ticket.gate,
                evidence: Vec::new(),
                model: OutcomeText::of("gate-ask-pending", Vec::new()),
                summary: pending_summary(&ticket.request),
            },
        }
    }
}

fn pending_summary(request: &ActionRequest) -> Summary {
    Summary {
        status: OutcomeText::of("turn-awaiting-approval", Vec::new()),
        action: request.context.tool.clone(),
        location: request.context.computer.clone(),
        result: OutcomeText::of("tool-empty", Vec::new()),
        verification: None,
        evidence_numbers: Vec::new(),
    }
}

#[must_use]
pub fn prepared_gate(gate: &GateRecord, answer: GateAnswer, source: JudgementSource) -> GateRecord {
    let mut record = gate.clone();
    record.source = source;
    record.outcome = match answer {
        GateAnswer::Allow => GateOutcome::Allow,
        GateAnswer::Deny => GateOutcome::Deny,
    };
    record
}

pub fn user_approved_run(
    ticket: &AskTicket,
    gate: GateRecord,
    computer: &dyn GatedComputer,
    view: &dyn ModelResultView,
    clock: &dyn Clock,
) -> ActionRun {
    let action = ticket.request.gate_action();
    let approved = approved_evidence(&ticket.request.context, &gate, JudgementSource::You, clock);
    let pass = Pass::issue(action.key(), gate.clone(), clock.now_ms());
    let mut run = executed_run(&ticket.request, &pass, computer, view, clock);
    run.gate = gate;
    run.summary
        .evidence_numbers
        .insert(0, approved.short.number());
    run.evidence.insert(0, approved);
    run
}

pub fn user_rejected_run(
    request: &ActionRequest,
    gate: GateRecord,
    view: &dyn ModelResultView,
    clock: &dyn Clock,
) -> ActionRun {
    let params = vec![("action".to_owned(), action_label(&request.subject))];
    let reason = ReasonPayload {
        code: "gate-user-rejected".to_owned(),
        params: params.clone(),
    };
    let evidence = rejected_evidence(&request.context, &gate, reason, clock);
    let model = view.rejection_text(&gate, &params);
    finish(gate, vec![evidence], model, &request.context)
}

pub fn denied_run(
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
            let evidence = build_evidence(&request.context, &gate, &result, clock);
            let model = view.result_text(&request.context.tool, &result);
            finish(gate, vec![evidence], model, &request.context)
        }
        Err(rejection) => failed_run(request, gate, &rejection, view, clock),
    }
}

fn perform(
    pass: &Pass,
    subject: &ActionSubject,
    computer: &dyn GatedComputer,
) -> Result<ActualResult, OpRejection> {
    match subject {
        ActionSubject::Command { line, timeout_ms } => {
            computer.run_command(pass, line, *timeout_ms)
        }
        ActionSubject::ReadFile { path } => computer.read_file(pass, &path.raw),
        ActionSubject::WriteFile { path, content } => computer.write_file(pass, &path.raw, content),
        ActionSubject::PathCheck { path } => computer.path_info(pass, &path.raw),
        ActionSubject::Capabilities => computer.capabilities(pass),
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
    let evidence = rejected_evidence(&request.context, &gate, reason.clone(), clock);
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
            "turn-not-executed",
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
        result: OutcomeText::of("tool-empty", Vec::new()),
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
        Some(ActionOutcome::Completed) => "turn-completed",
        Some(ActionOutcome::Failed) => "turn-failed",
        Some(ActionOutcome::Rejected) => "turn-rejected",
        Some(ActionOutcome::NotExecuted) | None => "turn-not-executed",
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
        ActionSubject::WriteFile { path, .. } => format!("write {}", path.raw),
        ActionSubject::PathCheck { path } => format!("path {}", path.raw),
        ActionSubject::Capabilities => "capabilities".to_owned(),
    }
}
