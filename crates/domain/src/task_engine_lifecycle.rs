use anybot_protocol::{
    BudgetItem, CardAnswer, DeviceReportKind, KnownTaskEventKind, TaskDetail, TaskStatus,
};

use crate::task_gate::finish_turn;
use crate::task_view::TaskView;

pub(crate) fn advance_goal(view: &mut TaskView, event: &KnownTaskEventKind) -> bool {
    match event {
        KnownTaskEventKind::TaskCreated { goal } => {
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
        | KnownTaskEventKind::TurnStarted
        | KnownTaskEventKind::TurnEnded { .. }
        | KnownTaskEventKind::TurnText { .. }
        | KnownTaskEventKind::ContextBuilt
        | KnownTaskEventKind::ModelSwapped { .. }
        | KnownTaskEventKind::ToolIntent { .. }
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
        | KnownTaskEventKind::DeviceReport { .. }
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
        | KnownTaskEventKind::TurnStarted
        | KnownTaskEventKind::TurnEnded { .. }
        | KnownTaskEventKind::TurnText { .. }
        | KnownTaskEventKind::ContextBuilt
        | KnownTaskEventKind::ModelSwapped { .. }
        | KnownTaskEventKind::ToolIntent { .. }
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
        | KnownTaskEventKind::DeviceReport { .. }
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
            view.status = TaskStatus::AwaitingUser;
            let reason = reason.clone();
            view.detail = Some(TaskDetail::GoalPaused { reason });
            true
        }
        KnownTaskEventKind::ComputerOffline => {
            view.status = TaskStatus::AwaitingUser;
            view.detail = Some(TaskDetail::ComputerOffline);
            true
        }
        KnownTaskEventKind::ModelUnreachable => {
            view.status = TaskStatus::Failed;
            view.detail = Some(TaskDetail::ModelUnreachable);
            true
        }
        KnownTaskEventKind::Stalled { minutes } => {
            view.status = TaskStatus::Stuck;
            view.detail = Some(TaskDetail::NoActivity { minutes: *minutes });
            true
        }
        KnownTaskEventKind::TaskCreated { .. }
        | KnownTaskEventKind::TaskCancelled { .. }
        | KnownTaskEventKind::TurnStarted
        | KnownTaskEventKind::TurnEnded { .. }
        | KnownTaskEventKind::TurnText { .. }
        | KnownTaskEventKind::ContextBuilt
        | KnownTaskEventKind::ModelSwapped { .. }
        | KnownTaskEventKind::ToolIntent { .. }
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
        | KnownTaskEventKind::DeviceReport { .. }
        | KnownTaskEventKind::DoneWhenChange { .. }
        | KnownTaskEventKind::DoneWhenAppliedByUser { .. } => false,
    }
}

pub(crate) fn advance_turn(view: &mut TaskView, event: &KnownTaskEventKind) -> bool {
    match event {
        KnownTaskEventKind::TurnStarted => {
            wake(view);
            view.budget.add_usage(BudgetItem::Turns, 1);
            view.evidence.turn_reply = false;
            crate::task_budget::refresh_budget_status(view);
            true
        }
        KnownTaskEventKind::TurnEnded { reply_delivered } => {
            view.evidence.turn_reply = *reply_delivered;
            finish_turn(view);
            true
        }
        KnownTaskEventKind::TurnText { .. }
        | KnownTaskEventKind::ContextBuilt
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
        | KnownTaskEventKind::DeviceReport { .. }
        | KnownTaskEventKind::DoneWhenChange { .. }
        | KnownTaskEventKind::DoneWhenAppliedByUser { .. } => false,
    }
}

pub(crate) fn wake(view: &mut TaskView) {
    if view.status == TaskStatus::Cancelled || view.status == TaskStatus::Completed {
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
