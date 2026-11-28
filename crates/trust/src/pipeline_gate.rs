use crate::evidence::rejected_evidence;
use crate::gate_flow::{GateConfig, judge};
use crate::op_ticket::OpTicket;
use crate::pipeline::ActionRequest;
use crate::pipeline_ask::AskTicket;
use crate::pipeline_call::{CallContext, prepared_action};
use crate::pipeline_exec::executed_run;
use crate::pipeline_run::{ActionRun, ModelResultView, RunStep, denied_run};
use crate::runner::GatedComputer;
use anybot_foundation::Clock;
use anybot_protocol::GateOutcome;

#[must_use]
#[derive(Debug)]
pub enum GateStep {
    Denied(ActionRun),
    Ask(Box<AskTicket>),
    Allow(OpTicket),
}

pub fn gate_step(
    request: &ActionRequest,
    ctx: &CallContext,
    computer: &dyn GatedComputer,
    config: &GateConfig,
    view: &dyn ModelResultView,
    clock: &dyn Clock,
) -> GateStep {
    let action = prepared_action(request, computer);
    let caps = computer.declared_caps();
    let verdict = judge(&action, &caps, config, computer.name());
    let gate = verdict.record();
    match verdict.outcome {
        GateOutcome::Deny => GateStep::Denied(denied_run(request, &verdict, &gate, view, clock)),
        GateOutcome::Ask => GateStep::Ask(Box::new(
            AskTicket::new(
                GateOutcome::Ask,
                gate.clone(),
                action,
                &caps,
                request,
                computer.name(),
                ctx,
            )
            .with_verdict(verdict),
        )),
        GateOutcome::Allow => {
            let fresh = prepared_action(request, computer);
            if fresh.subject != action.subject {
                return GateStep::Denied(denied_run(request, &verdict, &gate, view, clock));
            }
            let Some(ticket) = verdict.ticket(&fresh, computer.name(), &caps, clock.now_ms())
            else {
                return GateStep::Denied(denied_run(request, &verdict, &gate, view, clock));
            };
            GateStep::Allow(ticket)
        }
    }
}

#[must_use]
pub fn ask_declined_run(
    request: &ActionRequest,
    ticket: &AskTicket,
    view: &dyn ModelResultView,
    clock: &dyn Clock,
) -> ActionRun {
    let gate = ticket.gate();
    let reason = anybot_protocol::ReasonPayload {
        code: crate::message_codes::CODE_GATE_ASK_UNAVAILABLE.to_owned(),
        params: Vec::new(),
    };
    let evidence = rejected_evidence(&request.context, gate, reason, clock);
    let refusal = [(
        "refusal".to_owned(),
        crate::message_codes::CODE_GATE_ASK_DECLINED.to_owned(),
    )];
    let model = view.rejection_text(gate, &refusal);
    super::pipeline_exec::finish(gate.clone(), vec![evidence], model, &request.context)
}

pub fn exec_step(
    request: &ActionRequest,
    ticket: OpTicket,
    computer: &dyn GatedComputer,
    view: &dyn ModelResultView,
    clock: &dyn Clock,
) -> ActionRun {
    executed_run(request, ticket, computer, view, clock)
}

#[must_use]
pub fn run_action(
    request: &ActionRequest,
    ctx: &CallContext,
    computer: &dyn GatedComputer,
    config: &GateConfig,
    view: &dyn ModelResultView,
    clock: &dyn Clock,
) -> RunStep {
    match gate_step(request, ctx, computer, config, view, clock) {
        GateStep::Denied(run) => RunStep::Denied(run),
        GateStep::Ask(ticket) => RunStep::Ask(ticket),
        GateStep::Allow(ticket) => {
            RunStep::Executed(exec_step(request, ticket, computer, view, clock))
        }
    }
}
