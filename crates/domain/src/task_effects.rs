use anybot_protocol::{ReconcileOutcome, ToolEffect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResumeAction {
    Redo,
    Reconcile,
    Nothing,
    WaitUser,
}

#[must_use]
pub fn resume_action(level: ToolEffect, reconcile: Option<ReconcileOutcome>) -> ResumeAction {
    match (level, reconcile) {
        (ToolEffect::ReadOnly, _) | (ToolEffect::Repeatable, Some(ReconcileOutcome::NotDone)) => {
            ResumeAction::Redo
        }
        (ToolEffect::Repeatable, None) => ResumeAction::Reconcile,
        (ToolEffect::Repeatable, Some(ReconcileOutcome::Done)) => ResumeAction::Nothing,
        (ToolEffect::Repeatable, Some(ReconcileOutcome::Unknown))
        | (ToolEffect::Irreversible, _) => ResumeAction::WaitUser,
    }
}

#[cfg(test)]
#[path = "task_effects_tests.rs"]
mod tests;
