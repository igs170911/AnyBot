use crate::action::{GateAction, PathView};
use crate::card_data::{CardData, build_card};
use crate::command_target::{DeleteBase, command_targets};
use crate::evidence::ActionContext;
use crate::gate_flow::{GateConfig, judge};
use crate::op_ticket::OpTicket;
use crate::pipeline_exec::executed_run;
use crate::pipeline_run::{RunStep, denied_run, prepared_gate};
use crate::runner::GatedComputer;
use crate::safety_n5::{ActionClass, classes_of_command};
use anybot_foundation::Clock;
use anybot_protocol::{ComputerCaps, GateOutcome, JudgementSource};

use crate::action::ActionSubject;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestedAction {
    Command {
        line: String,
        timeout_ms: u64,
    },
    ReadFile {
        path: String,
    },
    WriteFile {
        path: String,
        content: String,
    },
    EditFile {
        path: String,
        old_text: String,
        new_text: String,
    },
    PathCheck {
        path: String,
    },
    Capabilities,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionRequest {
    pub context: ActionContext,
    pub subject: RequestedAction,
}

impl ActionRequest {
    #[must_use]
    pub fn gate_action(&self) -> GateAction {
        GateAction {
            tool: self.context.tool.clone(),
            subject: subject_of(&self.subject),
            extra_paths: Vec::new(),
            workdir: self.context.workdir.clone(),
        }
    }

    #[must_use]
    pub fn delete_targets(&self) -> Vec<PathView> {
        let RequestedAction::Command { line, .. } = &self.subject else {
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

fn subject_of(requested: &RequestedAction) -> ActionSubject {
    match requested {
        RequestedAction::Command { line, timeout_ms } => ActionSubject::Command {
            line: line.clone(),
            timeout_ms: *timeout_ms,
        },
        RequestedAction::ReadFile { path } => ActionSubject::ReadFile {
            path: PathView::raw(path),
        },
        RequestedAction::WriteFile { path, content } => ActionSubject::WriteFile {
            path: PathView::raw(path),
            content: content.clone(),
        },
        RequestedAction::EditFile {
            path,
            old_text,
            new_text,
        } => ActionSubject::EditFile {
            path: PathView::raw(path),
            old_text: old_text.clone(),
            new_text: new_text.clone(),
        },
        RequestedAction::PathCheck { path } => ActionSubject::PathCheck {
            path: PathView::raw(path),
        },
        RequestedAction::Capabilities => ActionSubject::Capabilities,
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
            GateOutcome::Ask,
            gate.clone(),
            action,
            &caps,
            request,
            computer.name(),
        ))),
        GateOutcome::Allow => {
            let fresh = prepared_action(request, computer);
            if fresh.subject != action.subject {
                return RunStep::Denied(denied_run(request, &verdict, &gate, view, clock));
            }
            let Some(ticket) = verdict.ticket(&fresh, computer.name(), clock.now_ms()) else {
                return RunStep::Denied(denied_run(request, &verdict, &gate, view, clock));
            };
            RunStep::Executed(executed_run(request, ticket, computer, view, clock))
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
        outcome: GateOutcome,
        gate: GateRecord,
        action: GateAction,
        caps: &ComputerCaps,
        request: &ActionRequest,
        computer: &str,
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
    action.subject =
        crate::path_resolve::resolve_subject(action.subject, computer, &action.workdir);
    action.extra_paths =
        crate::path_resolve::resolved_targets(&request.delete_targets(), computer, &action.workdir);
    action
}
