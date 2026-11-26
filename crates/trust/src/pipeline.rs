use crate::action::{GateAction, PathView};
use crate::card_data::{CardData, build_card};
use crate::command_target::command_target_paths;
use crate::evidence::ActionContext;
use crate::gate_flow::{GateConfig, judge};
use crate::pipeline_run::{RunStep, denied_run, executed_run, prepared_gate};
use crate::runner::GatedComputer;
use crate::safety_n5::{ActionClass, classes_of_command};
use anybot_foundation::Clock;
use anybot_protocol::{GateOutcome, JudgementSource};

use crate::action::ActionSubject;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionRequest {
    pub context: ActionContext,
    pub subject: ActionSubject,
}

impl ActionRequest {
    #[must_use]
    pub fn gate_action(&self) -> GateAction {
        GateAction {
            tool: self.context.tool.clone(),
            subject: self.subject.clone(),
            extra_paths: Vec::new(),
            workdir: self.context.workdir.clone(),
        }
    }

    #[must_use]
    pub fn delete_targets(&self) -> Vec<PathView> {
        let ActionSubject::Command { line, .. } = &self.subject else {
            return Vec::new();
        };
        if !classes_of_command(line).contains(&ActionClass::Delete) {
            return Vec::new();
        }
        command_target_paths(line)
            .into_iter()
            .map(|token| PathView::raw(&token))
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateAnswer {
    Allow,
    Deny,
}

#[must_use]
pub fn run_action(
    request: &ActionRequest,
    computer: &dyn GatedComputer,
    config: &GateConfig,
    view: &dyn crate::pipeline_run::ModelResultView,
    clock: &dyn Clock,
) -> RunStep {
    let action = prepared_action(request, computer);
    let caps = computer.declared_caps();
    let verdict = judge(&action, &caps, config);
    let gate = verdict.record();
    match verdict.outcome {
        GateOutcome::Deny => RunStep::Denied(denied_run(request, &verdict, &gate, view, clock)),
        GateOutcome::Ask => RunStep::Ask(AskTicket {
            gate,
            card: build_card(&action, &caps),
            request: request.clone(),
        }),
        GateOutcome::Allow => {
            let Some(pass) = verdict.pass(&action, clock.now_ms()) else {
                return RunStep::Denied(denied_run(request, &verdict, &gate, view, clock));
            };
            RunStep::Executed(executed_run(request, &pass, computer, view, clock))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AskTicket {
    pub gate: GateRecord,
    pub card: CardData,
    pub request: ActionRequest,
}

use anybot_protocol::GateRecord;

pub fn answer_ask(
    ticket: &AskTicket,
    answer: GateAnswer,
    computer: &dyn GatedComputer,
    view: &dyn crate::pipeline_run::ModelResultView,
    clock: &dyn Clock,
) -> crate::pipeline_run::ActionRun {
    let gate = prepared_gate(&ticket.gate, answer, JudgementSource::You);
    match answer {
        GateAnswer::Deny => denied_answer(ticket, gate, view, clock),
        GateAnswer::Allow => approved_answer(ticket, gate, computer, view, clock),
    }
}

fn denied_answer(
    ticket: &AskTicket,
    gate: GateRecord,
    view: &dyn crate::pipeline_run::ModelResultView,
    clock: &dyn Clock,
) -> crate::pipeline_run::ActionRun {
    crate::pipeline_run::user_rejected_run(&ticket.request, gate, view, clock)
}

fn approved_answer(
    ticket: &AskTicket,
    gate: GateRecord,
    computer: &dyn GatedComputer,
    view: &dyn crate::pipeline_run::ModelResultView,
    clock: &dyn Clock,
) -> crate::pipeline_run::ActionRun {
    crate::pipeline_run::user_approved_run(ticket, gate, computer, view, clock)
}

fn prepared_action(request: &ActionRequest, computer: &dyn GatedComputer) -> GateAction {
    let mut action = request.gate_action();
    action.extra_paths = resolved_targets(&request.delete_targets(), computer, &action.workdir);
    action
}

fn resolved_targets(
    raw: &[PathView],
    computer: &dyn GatedComputer,
    workdir: &str,
) -> Vec<PathView> {
    raw.iter()
        .map(|view| {
            let effective = view.effective(workdir);
            match computer.resolve_target(&effective) {
                Some(canonical) => PathView::resolved(&view.raw, &canonical),
                None => PathView::raw(&view.raw),
            }
        })
        .collect()
}
