use crate::action::{GateAction, PathView};
use crate::card_data::{CardData, build_card};
use crate::command_target::{DeleteBase, command_targets};
use crate::evidence::ActionContext;
use crate::gate_flow::{GateConfig, GateVerdict, judge};
use crate::op_ticket::OpTicket;
use crate::pipeline_exec::executed_run;
use crate::pipeline_run::{RunStep, denied_run, prepared_gate};
use crate::runner::GatedComputer;
use crate::safety_n5::{ActionClass, classes_of_command};
use anybot_foundation::Clock;
use anybot_protocol::{ComputerCaps, GateOutcome, JudgementSource};

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
        command_targets(line)
            .into_iter()
            .map(|target| view_of_target(&target))
            .collect()
    }
}

fn view_of_target(target: &crate::command_target::DeleteTarget) -> PathView {
    if crate::command_target::has_expansion_token(&target.token)
        || target.base == DeleteBase::Unresolved
    {
        return PathView::unresolvable(&target.token);
    }
    if target.base == DeleteBase::Cd && !target.token.starts_with('/') {
        return PathView::raw(&format!("{}/{}", target.cd_path, target.token));
    }
    PathView::raw(&target.token)
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
    let verdict = judge(&action, &caps, config, computer.name());
    let gate = verdict.record();
    match verdict.outcome {
        GateOutcome::Deny => RunStep::Denied(denied_run(request, &verdict, &gate, view, clock)),
        GateOutcome::Ask => RunStep::Ask(Box::new(AskTicket::new(
            verdict,
            action,
            &caps,
            request,
            computer.name(),
        ))),
        GateOutcome::Allow => {
            let Some(ticket) = verdict.ticket(&action, computer.name(), clock.now_ms()) else {
                return RunStep::Denied(denied_run(request, &verdict, &gate, view, clock));
            };
            RunStep::Executed(executed_run(request, ticket, computer, view, clock))
        }
    }
}

#[derive(Debug)]
pub struct AskTicket {
    gate: GateRecord,
    verdict: GateVerdict,
    action: GateAction,
    card: CardData,
    request: ActionRequest,
    computer: String,
    caps: ComputerCaps,
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
        view: &dyn crate::pipeline_run::ModelResultView,
        clock: &dyn Clock,
    ) -> crate::pipeline_run::ActionRun {
        self.finish(answer, computer, view, clock)
    }

    pub(crate) fn new(
        verdict: GateVerdict,
        action: GateAction,
        caps: &ComputerCaps,
        request: &ActionRequest,
        computer: &str,
    ) -> Self {
        let card = build_card(&action, caps);
        AskTicket {
            gate: verdict.record(),
            verdict,
            action,
            card,
            request: request.clone(),
            computer: computer.to_owned(),
            caps: caps.clone(),
        }
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
        view: &dyn crate::pipeline_run::ModelResultView,
        clock: &dyn Clock,
    ) -> crate::pipeline_run::ActionRun {
        if self.verdict.outcome != GateOutcome::Ask {
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
                let ticket = OpTicket::issue(
                    &self.action.subject,
                    self.action.key(),
                    gate.clone(),
                    computer.name(),
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

use anybot_protocol::GateRecord;

pub fn answer_ask(
    ticket: AskTicket,
    answer: GateAnswer,
    computer: &dyn GatedComputer,
    view: &dyn crate::pipeline_run::ModelResultView,
    clock: &dyn Clock,
) -> crate::pipeline_run::ActionRun {
    ticket.answer(answer, computer, view, clock)
}

fn prepared_action(request: &ActionRequest, computer: &dyn GatedComputer) -> GateAction {
    let mut action = request.gate_action();
    let base = computer.declared_workdir();
    action.workdir = base;
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
            if !view.resolvable {
                return view.clone();
            }
            let effective = view.effective(workdir);
            match computer.resolve_target(&effective) {
                Some(canonical) => PathView::resolved(&view.raw, &canonical),
                None => PathView::unresolvable(&view.raw),
            }
        })
        .collect()
}
