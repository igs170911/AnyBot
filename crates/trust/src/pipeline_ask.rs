use crate::action::GateAction;
use crate::card_data::{CardData, build_card};
use crate::op_ticket::OpTicket;
use crate::pipeline::{ActionRequest, GateAnswer};
use crate::pipeline_run::{ActionRun, ModelResultView, prepared_gate};
use crate::runner::GatedComputer;
use anybot_foundation::{CallId, Clock, ContentDigest, TurnId};
use anybot_protocol::{ComputerCaps, GateOutcome, GateRecord, JudgementSource};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallContext {
    pub call: CallId,
    pub turn: TurnId,
    pub provider: String,
}

impl CallContext {
    #[must_use]
    pub fn new(call: CallId, turn: TurnId, provider: &str) -> Self {
        CallContext {
            call,
            turn,
            provider: provider.to_owned(),
        }
    }
}

#[derive(Debug)]
pub struct AskTicket {
    gate: GateRecord,
    outcome: GateOutcome,
    action: GateAction,
    card: CardData,
    request: ActionRequest,
    computer: String,
    caps: ComputerCaps,
    call: CallId,
    turn: TurnId,
    provider: String,
}

impl AskTicket {
    #[must_use]
    pub fn card(&self) -> &CardData {
        &self.card
    }

    #[must_use]
    pub fn gate(&self) -> &GateRecord {
        &self.gate
    }

    pub fn answer(
        self,
        answer: GateAnswer,
        computer: &dyn GatedComputer,
        view: &dyn ModelResultView,
        clock: &dyn Clock,
    ) -> ActionRun {
        self.finish(answer, computer, view, clock)
    }

    pub(super) fn new(
        outcome: GateOutcome,
        gate: GateRecord,
        action: GateAction,
        caps: &ComputerCaps,
        request: &ActionRequest,
        computer: &str,
        ctx: &CallContext,
    ) -> Self {
        let card = build_card(&action, caps);
        AskTicket {
            gate,
            outcome,
            action,
            card,
            request: request.clone(),
            computer: computer.to_owned(),
            caps: caps.clone(),
            call: ctx.call,
            turn: ctx.turn,
            provider: ctx.provider.clone(),
        }
    }

    #[must_use]
    pub fn card_digest(&self) -> String {
        let text = format!(
            "{}|{}|{}|{}|{}",
            self.gate.rule,
            self.gate.gate.number(),
            self.turn,
            self.call,
            self.provider
        );
        let bytes = ContentDigest::sha256(text.as_bytes());
        let full = bytes.hex();
        full.get(..12)
            .map_or_else(|| full.to_owned(), ToOwned::to_owned)
    }

    #[must_use]
    pub fn computer_name(&self) -> &str {
        &self.computer
    }

    #[must_use]
    pub fn request(&self) -> &ActionRequest {
        &self.request
    }

    fn finish(
        self,
        answer: GateAnswer,
        computer: &dyn GatedComputer,
        view: &dyn ModelResultView,
        clock: &dyn Clock,
    ) -> ActionRun {
        if self.outcome != GateOutcome::Ask {
            return crate::pipeline_run::user_rejected_run(&self.request, self.gate, view, clock);
        }
        if computer.name() != self.computer || computer.declared_caps() != self.caps {
            let gate = prepared_gate(&self.gate, GateAnswer::Deny, JudgementSource::ProgramCheck);
            return crate::pipeline_run::ask_computer_mismatch_run(
                &self.request,
                gate,
                &self.computer,
                computer.name(),
                view,
                clock,
            );
        }
        let gate = prepared_gate(&self.gate, answer, JudgementSource::You);
        match answer {
            GateAnswer::Deny => {
                crate::pipeline_run::user_rejected_run(&self.request, gate, view, clock)
            }
            GateAnswer::Allow => {
                let action = prepared_action(&self.request, computer);
                if action.subject != self.action.subject {
                    let gate =
                        prepared_gate(&self.gate, GateAnswer::Deny, JudgementSource::ProgramCheck);
                    return crate::pipeline_run::ask_computer_mismatch_run(
                        &self.request,
                        gate,
                        &self.computer,
                        computer.name(),
                        view,
                        clock,
                    );
                }
                let ticket = OpTicket::issue(
                    &action.subject,
                    action.key(),
                    gate.clone(),
                    computer.name(),
                    &computer.declared_caps(),
                    &action.workdir,
                    clock.now_ms(),
                );
                crate::pipeline_run::user_approved_run(
                    &self.request,
                    &gate,
                    ticket,
                    computer,
                    view,
                    clock,
                )
            }
        }
    }
}

pub fn answer_ask(
    ticket: &AskTicket,
    answer: GateAnswer,
    computer: &dyn GatedComputer,
    view: &dyn ModelResultView,
    clock: &dyn Clock,
) -> ActionRun {
    ticket.clone_ticket().answer(answer, computer, view, clock)
}

pub(crate) fn prepared_action(request: &ActionRequest, computer: &dyn GatedComputer) -> GateAction {
    let mut action = request.gate_action();
    let base = computer.declared_workdir();
    action.workdir = base;
    action.subject =
        crate::path_resolve::resolve_subject(action.subject, computer, &action.workdir);
    action.extra_paths =
        crate::path_resolve::resolved_targets(&request.delete_targets(), computer, &action.workdir);
    action
}

impl AskTicket {
    pub(crate) fn clone_ticket(&self) -> Self {
        AskTicket {
            gate: self.gate.clone(),
            outcome: self.outcome,
            action: self.action.clone(),
            card: self.card.clone(),
            request: self.request.clone(),
            computer: self.computer.clone(),
            caps: self.caps.clone(),
            call: self.call,
            turn: self.turn,
            provider: self.provider.clone(),
        }
    }
}
