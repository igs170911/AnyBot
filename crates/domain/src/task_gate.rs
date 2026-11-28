use anybot_protocol::{TaskDetail, TaskStatus, TurnEndReason};

use crate::task_view::TaskView;

pub fn finish_turn(view: &mut TaskView) {
    if is_terminal(view.status) {
        return;
    }
    if view.over_budget() {
        view.set_over_budget_detail();
        return;
    }
    if !view.unresolved_unknowns.is_empty() {
        view.status = TaskStatus::AwaitingUser;
        view.detail = Some(TaskDetail::ResumePending);
        return;
    }
    if view.gate.acceptance_unavailable {
        view.status = TaskStatus::Unaccepted;
        view.detail = Some(TaskDetail::NoVisibleResult);
        return;
    }
    if view.gate.step_rejected {
        view.status = TaskStatus::Unaccepted;
        view.detail = Some(TaskDetail::StepRejected);
        return;
    }
    if view.gate.last_write_ok == Some(false) && view.criteria.is_empty() {
        view.status = TaskStatus::Unaccepted;
        view.detail = Some(TaskDetail::AcceptanceFailed);
        return;
    }
    if view.unverified_actions() {
        view.status = TaskStatus::Unaccepted;
        view.detail = Some(TaskDetail::UnverifiedActions);
        return;
    }
    if view.completion_ready() {
        view.complete();
        return;
    }
    view.set_running(TaskDetail::CurrentStep {
        step: view.goal.clone(),
    });
}

fn is_terminal(status: TaskStatus) -> bool {
    matches!(status, TaskStatus::Cancelled | TaskStatus::Stopped)
}

pub fn end_turn(view: &mut TaskView, reason: &TurnEndReason, reply_delivered: bool) {
    if is_terminal(view.status) {
        return;
    }
    match reason {
        TurnEndReason::Answered => {
            view.evidence.turn_reply = reply_delivered;
            finish_turn(view);
        }
        TurnEndReason::ContextBudget { .. } => {
            view.status = TaskStatus::Stuck;
            view.detail = Some(TaskDetail::ContextBudgetInsufficient);
        }
        TurnEndReason::PendingApproval => {
            view.status = TaskStatus::AwaitingUser;
            if !matches!(view.detail, Some(TaskDetail::QuestionPending { .. })) {
                let action = pending_action(view);
                view.detail = Some(TaskDetail::ApprovalPending { action });
            }
        }
        TurnEndReason::ModelFailed { code } => {
            view.status = TaskStatus::Failed;
            let reason = code.clone();
            view.detail = Some(TaskDetail::Error { reason });
        }
    }
}

impl TaskView {
    #[must_use]
    pub fn over_budget(&self) -> bool {
        self.budget.over.is_some()
    }

    pub fn set_over_budget_detail(&mut self) {
        if let Some(over) = &self.budget.over {
            self.status = TaskStatus::Stuck;
            self.detail = Some(TaskDetail::OverBudget {
                item: over.item,
                used: over.used,
                limit: over.limit,
            });
        }
    }
}

#[cfg(test)]
#[path = "task_gate_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "task_gate_budget_tests.rs"]
mod budget_tests;

#[cfg(test)]
#[path = "task_state_prop_tests.rs"]
mod state_props;

fn pending_action(view: &TaskView) -> String {
    match &view.detail {
        Some(TaskDetail::ApprovalPending { action }) => action.clone(),
        _ => view.goal.clone(),
    }
}
