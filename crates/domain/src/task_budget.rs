use anybot_protocol::{TaskDetail, TaskStatus};

use crate::task_view::TaskView;

pub fn refresh_budget_status(view: &mut TaskView) {
    if view.over_budget() {
        view.set_over_budget_detail();
        return;
    }
    let was_over = matches!(view.detail, Some(TaskDetail::OverBudget { .. }));
    let stuck = view.status == TaskStatus::Stuck;
    if was_over && stuck {
        view.set_running(TaskDetail::CurrentStep {
            step: view.goal.clone(),
        });
    }
}

#[cfg(test)]
#[path = "task_budget_tests.rs"]
mod tests;
