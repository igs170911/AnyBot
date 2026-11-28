use anybot_protocol::{CardAnswer, DeviceReportKind, TaskDetail, TaskStatus};

use crate::task_view::TaskView;

pub(crate) fn wake(view: &mut TaskView) {
    let can_wake = match view.status {
        TaskStatus::AwaitingUser
        | TaskStatus::Failed
        | TaskStatus::Stopped
        | TaskStatus::Completed
        | TaskStatus::Unaccepted => true,
        TaskStatus::Stuck => !view.over_budget(),
        TaskStatus::Running | TaskStatus::Cancelled => false,
    };
    if can_wake || matches!(view.detail, Some(TaskDetail::AutoResuming)) {
        let step = view.goal.clone();
        view.set_running(TaskDetail::CurrentStep { step });
    }
}

pub(crate) fn answer_card(view: &mut TaskView, answer: CardAnswer) {
    if matches!(view.status, TaskStatus::Cancelled | TaskStatus::Stopped) {
        return;
    }
    if answer == CardAnswer::Deny {
        view.gate.step_rejected = true;
    }
    let step = view.goal.clone();
    view.set_running(TaskDetail::CurrentStep { step });
}

pub(crate) fn report_device(view: &mut TaskView, kind: DeviceReportKind) {
    match kind {
        DeviceReportKind::Shown | DeviceReportKind::Opened => {
            view.evidence.delivered_reported = true;
        }
    }
    view.upgrade_completed_detail();
}
pub(crate) fn pause(view: &mut TaskView, reason: &str) {
    view.status = TaskStatus::AwaitingUser;
    let reason = reason.to_owned();
    view.detail = Some(TaskDetail::GoalPaused { reason });
}

pub(crate) fn offline(view: &mut TaskView) {
    view.status = TaskStatus::AwaitingUser;
    view.detail = Some(TaskDetail::ComputerOffline);
}

pub(crate) fn unreachable(view: &mut TaskView) {
    if matches!(view.status, TaskStatus::Cancelled | TaskStatus::Stopped) {
        return;
    }
    view.status = TaskStatus::Stuck;
    view.detail = Some(TaskDetail::ModelUnreachable);
}

pub(crate) fn stalled(view: &mut TaskView, minutes: u32) {
    view.status = TaskStatus::Stuck;
    view.detail = Some(TaskDetail::NoActivity { minutes });
}

pub(crate) fn start_verification(view: &mut TaskView) {
    if matches!(view.status, TaskStatus::Cancelled | TaskStatus::Stopped) {
        return;
    }
    for item in &mut view.criteria {
        item.passed = None;
    }
    view.set_running(TaskDetail::Verifying);
}
