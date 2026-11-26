use crate::evidence::{approved_evidence, rejected_evidence};
use crate::gate_flow::{GateVerdict, rule_code};
use crate::op_ticket::OpTicket;
use crate::pipeline::{ActionRequest, AskTicket, GateAnswer};
use crate::pipeline_exec::executed_run;
use crate::pipeline_exec::{action_label, finish};
use crate::runner::{GatedComputer, OpRejection};
use anybot_foundation::Clock;
use anybot_protocol::{
    ActualResult, EvidenceRecord, GateOutcome, GateRecord, JudgementSource, ReasonPayload,
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

#[derive(Debug, PartialEq, Eq)]
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
    ticket: OpTicket,
    computer: &dyn GatedComputer,
    view: &dyn ModelResultView,
    clock: &dyn Clock,
) -> ActionRun {
    let approved = approved_evidence(&request.context, gate, JudgementSource::You, clock);
    let mut run = executed_run(request, ticket, computer, view, clock);
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

pub(crate) fn ask_computer_mismatch_run(
    request: &ActionRequest,
    gate: GateRecord,
    asked_for: &str,
    answered_on: &str,
    view: &dyn ModelResultView,
    clock: &dyn Clock,
) -> ActionRun {
    let params = vec![
        ("asked_for".to_owned(), asked_for.to_owned()),
        ("answered_on".to_owned(), answered_on.to_owned()),
    ];
    let reason = ReasonPayload {
        code: crate::message_codes::CODE_GATE_ASK_COMPUTER_MISMATCH.to_owned(),
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
