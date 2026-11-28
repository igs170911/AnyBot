use anybot_protocol::{CardAnswer, DeviceReportKind, KnownTaskEventKind, TaskDetail, TaskStatus};

use crate::task_view::TaskView;

pub(crate) fn advance_goal(view: &mut TaskView, event: &KnownTaskEventKind) -> bool {
    match event {
        KnownTaskEventKind::TaskCreated { goal, .. } => {
            view.goal.clone_from(goal);
            let step = view.goal.clone();
            view.set_running(TaskDetail::CurrentStep { step });
            true
        }
        KnownTaskEventKind::TaskCancelled { .. }
        | KnownTaskEventKind::TaskStopped
        | KnownTaskEventKind::Wake { .. }
        | KnownTaskEventKind::GoalPaused { .. }
        | KnownTaskEventKind::ComputerOffline
        | KnownTaskEventKind::ModelUnreachable
        | KnownTaskEventKind::Stalled { .. }
        | KnownTaskEventKind::TurnStarted { .. }
        | KnownTaskEventKind::TurnEnded { .. }
        | KnownTaskEventKind::ContextBuilt { .. }
        | KnownTaskEventKind::ModelAsked { .. }
        | KnownTaskEventKind::ModelAnswer { .. }
        | KnownTaskEventKind::ModelUsage { .. }
        | KnownTaskEventKind::ToolNotRun { .. }
        | KnownTaskEventKind::ModelSwapped { .. }
        | KnownTaskEventKind::ToolIntent { .. }
        | KnownTaskEventKind::GateResult { .. }
        | KnownTaskEventKind::ToolStarted { .. }
        | KnownTaskEventKind::ToolCompleted { .. }
        | KnownTaskEventKind::ToolFailed { .. }
        | KnownTaskEventKind::OutcomeUnknown { .. }
        | KnownTaskEventKind::Reconciled { .. }
        | KnownTaskEventKind::ActionVerified { .. }
        | KnownTaskEventKind::EvidenceAdded { .. }
        | KnownTaskEventKind::CriterionPassed { .. }
        | KnownTaskEventKind::CriterionFailed { .. }
        | KnownTaskEventKind::AcceptanceUnavailable
        | KnownTaskEventKind::ClaimChecked { .. }
        | KnownTaskEventKind::BudgetUsage { .. }
        | KnownTaskEventKind::BudgetLimit { .. }
        | KnownTaskEventKind::CardShown { .. }
        | KnownTaskEventKind::CardAnswered { .. }
        | KnownTaskEventKind::QuestionShown { .. }
        | KnownTaskEventKind::QuestionAnswered { .. }
        | KnownTaskEventKind::DeviceReport { .. }
        | KnownTaskEventKind::DeliveryReport { .. }
        | KnownTaskEventKind::DoneWhenChange { .. }
        | KnownTaskEventKind::DoneWhenAppliedByUser { .. } => false,
    }
}

pub(crate) fn advance_cancel(view: &mut TaskView, event: &KnownTaskEventKind) -> bool {
    match event {
        KnownTaskEventKind::TaskCancelled { reason } => {
            view.status = TaskStatus::Cancelled;
            let reason = reason.clone();
            view.detail = Some(TaskDetail::CancelledReason { reason });
            true
        }
        KnownTaskEventKind::TaskCreated { .. }
        | KnownTaskEventKind::TaskStopped
        | KnownTaskEventKind::Wake { .. }
        | KnownTaskEventKind::GoalPaused { .. }
        | KnownTaskEventKind::ComputerOffline
        | KnownTaskEventKind::ModelUnreachable
        | KnownTaskEventKind::Stalled { .. }
        | KnownTaskEventKind::TurnStarted { .. }
        | KnownTaskEventKind::TurnEnded { .. }
        | KnownTaskEventKind::ContextBuilt { .. }
        | KnownTaskEventKind::ModelAsked { .. }
        | KnownTaskEventKind::ModelAnswer { .. }
        | KnownTaskEventKind::ModelUsage { .. }
        | KnownTaskEventKind::ToolNotRun { .. }
        | KnownTaskEventKind::ModelSwapped { .. }
        | KnownTaskEventKind::ToolIntent { .. }
        | KnownTaskEventKind::GateResult { .. }
        | KnownTaskEventKind::ToolStarted { .. }
        | KnownTaskEventKind::ToolCompleted { .. }
        | KnownTaskEventKind::ToolFailed { .. }
        | KnownTaskEventKind::OutcomeUnknown { .. }
        | KnownTaskEventKind::Reconciled { .. }
        | KnownTaskEventKind::ActionVerified { .. }
        | KnownTaskEventKind::EvidenceAdded { .. }
        | KnownTaskEventKind::CriterionPassed { .. }
        | KnownTaskEventKind::CriterionFailed { .. }
        | KnownTaskEventKind::AcceptanceUnavailable
        | KnownTaskEventKind::ClaimChecked { .. }
        | KnownTaskEventKind::BudgetUsage { .. }
        | KnownTaskEventKind::BudgetLimit { .. }
        | KnownTaskEventKind::CardShown { .. }
        | KnownTaskEventKind::CardAnswered { .. }
        | KnownTaskEventKind::QuestionShown { .. }
        | KnownTaskEventKind::QuestionAnswered { .. }
        | KnownTaskEventKind::DeviceReport { .. }
        | KnownTaskEventKind::DeliveryReport { .. }
        | KnownTaskEventKind::DoneWhenChange { .. }
        | KnownTaskEventKind::DoneWhenAppliedByUser { .. } => false,
    }
}

pub(crate) fn advance_lifecycle(view: &mut TaskView, event: &KnownTaskEventKind) -> bool {
    match event {
        KnownTaskEventKind::TaskStopped => {
            view.status = TaskStatus::Stopped;
            view.detail = Some(TaskDetail::YouStopped);
            true
        }
        KnownTaskEventKind::Wake { .. } => {
            wake(view);
            true
        }
        KnownTaskEventKind::GoalPaused { reason } => {
            pause(view, reason);
            true
        }
        KnownTaskEventKind::ComputerOffline => {
            offline(view);
            true
        }
        KnownTaskEventKind::ModelUnreachable => {
            unreachable(view);
            true
        }
        KnownTaskEventKind::Stalled { minutes } => {
            stalled(view, *minutes);
            true
        }
        KnownTaskEventKind::TaskCreated { .. }
        | KnownTaskEventKind::TaskCancelled { .. }
        | KnownTaskEventKind::TurnStarted { .. }
        | KnownTaskEventKind::TurnEnded { .. }
        | KnownTaskEventKind::ContextBuilt { .. }
        | KnownTaskEventKind::ModelAsked { .. }
        | KnownTaskEventKind::ModelAnswer { .. }
        | KnownTaskEventKind::ModelUsage { .. }
        | KnownTaskEventKind::ToolNotRun { .. }
        | KnownTaskEventKind::ModelSwapped { .. }
        | KnownTaskEventKind::ToolIntent { .. }
        | KnownTaskEventKind::GateResult { .. }
        | KnownTaskEventKind::ToolStarted { .. }
        | KnownTaskEventKind::ToolCompleted { .. }
        | KnownTaskEventKind::ToolFailed { .. }
        | KnownTaskEventKind::OutcomeUnknown { .. }
        | KnownTaskEventKind::Reconciled { .. }
        | KnownTaskEventKind::ActionVerified { .. }
        | KnownTaskEventKind::EvidenceAdded { .. }
        | KnownTaskEventKind::CriterionPassed { .. }
        | KnownTaskEventKind::CriterionFailed { .. }
        | KnownTaskEventKind::AcceptanceUnavailable
        | KnownTaskEventKind::ClaimChecked { .. }
        | KnownTaskEventKind::BudgetUsage { .. }
        | KnownTaskEventKind::BudgetLimit { .. }
        | KnownTaskEventKind::CardShown { .. }
        | KnownTaskEventKind::CardAnswered { .. }
        | KnownTaskEventKind::QuestionShown { .. }
        | KnownTaskEventKind::QuestionAnswered { .. }
        | KnownTaskEventKind::DeviceReport { .. }
        | KnownTaskEventKind::DeliveryReport { .. }
        | KnownTaskEventKind::DoneWhenChange { .. }
        | KnownTaskEventKind::DoneWhenAppliedByUser { .. } => false,
    }
}

pub(crate) fn advance_turn(view: &mut TaskView, event: &KnownTaskEventKind) -> bool {
    match event {
        KnownTaskEventKind::TurnStarted { .. } => {
            wake(view);
            crate::task_budget::begin_turn(view);
            true
        }
        KnownTaskEventKind::TurnEnded {
            reason,
            reply_delivered,
        } => {
            crate::task_gate::end_turn(view, reason, *reply_delivered);
            true
        }
        KnownTaskEventKind::ContextBuilt { .. }
        | KnownTaskEventKind::ModelAsked { .. }
        | KnownTaskEventKind::ModelAnswer { .. }
        | KnownTaskEventKind::ModelUsage { .. }
        | KnownTaskEventKind::ModelSwapped { .. } => true,
        KnownTaskEventKind::TaskCreated { .. }
        | KnownTaskEventKind::TaskCancelled { .. }
        | KnownTaskEventKind::TaskStopped
        | KnownTaskEventKind::Wake { .. }
        | KnownTaskEventKind::GoalPaused { .. }
        | KnownTaskEventKind::ComputerOffline
        | KnownTaskEventKind::ModelUnreachable
        | KnownTaskEventKind::Stalled { .. }
        | KnownTaskEventKind::ToolIntent { .. }
        | KnownTaskEventKind::GateResult { .. }
        | KnownTaskEventKind::ToolStarted { .. }
        | KnownTaskEventKind::ToolCompleted { .. }
        | KnownTaskEventKind::ToolFailed { .. }
        | KnownTaskEventKind::OutcomeUnknown { .. }
        | KnownTaskEventKind::Reconciled { .. }
        | KnownTaskEventKind::ActionVerified { .. }
        | KnownTaskEventKind::EvidenceAdded { .. }
        | KnownTaskEventKind::CriterionPassed { .. }
        | KnownTaskEventKind::CriterionFailed { .. }
        | KnownTaskEventKind::AcceptanceUnavailable
        | KnownTaskEventKind::ClaimChecked { .. }
        | KnownTaskEventKind::BudgetUsage { .. }
        | KnownTaskEventKind::BudgetLimit { .. }
        | KnownTaskEventKind::CardShown { .. }
        | KnownTaskEventKind::CardAnswered { .. }
        | KnownTaskEventKind::QuestionShown { .. }
        | KnownTaskEventKind::QuestionAnswered { .. }
        | KnownTaskEventKind::DeviceReport { .. }
        | KnownTaskEventKind::DeliveryReport { .. }
        | KnownTaskEventKind::DoneWhenChange { .. }
        | KnownTaskEventKind::DoneWhenAppliedByUser { .. }
        | KnownTaskEventKind::ToolNotRun { .. } => false,
    }
}

pub(crate) fn wake(view: &mut TaskView) {
    if matches!(
        view.status,
        TaskStatus::Cancelled | TaskStatus::Completed | TaskStatus::Unaccepted
    ) {
        let step = view.goal.clone();
        view.set_running(TaskDetail::CurrentStep { step });
    }
}

pub(crate) fn answer_card(view: &mut TaskView, answer: CardAnswer) {
    if answer == CardAnswer::Deny {
        view.gate.step_rejected = true;
    }
    let step = view.goal.clone();
    view.set_running(TaskDetail::CurrentStep { step });
}

pub(crate) fn report_device(view: &mut TaskView, kind: DeviceReportKind) {
    match kind {
        DeviceReportKind::Shown => view.evidence.delivered_reported = true,
        DeviceReportKind::ThumbsUp | DeviceReportKind::Opened => view.report.user_confirmed = true,
        DeviceReportKind::ThumbsDown => {}
    }
    view.upgrade_completed_detail();
}

fn pause(view: &mut TaskView, reason: &str) {
    view.status = TaskStatus::AwaitingUser;
    let reason = reason.to_owned();
    view.detail = Some(TaskDetail::GoalPaused { reason });
}

fn offline(view: &mut TaskView) {
    view.status = TaskStatus::AwaitingUser;
    view.detail = Some(TaskDetail::ComputerOffline);
}

fn unreachable(view: &mut TaskView) {
    view.status = TaskStatus::Stuck;
    view.detail = Some(TaskDetail::ModelUnreachable);
}

fn stalled(view: &mut TaskView, minutes: u32) {
    view.status = TaskStatus::Stuck;
    view.detail = Some(TaskDetail::NoActivity { minutes });
}
