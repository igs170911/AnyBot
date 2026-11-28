use anybot_protocol::KnownTaskEventKind;

use crate::state::StateBuilder;

pub(crate) fn apply_notes(b: &mut StateBuilder, seq: u64, known: &KnownTaskEventKind) {
    match known {
        KnownTaskEventKind::EvidenceAdded { record } => {
            b.evidence.push((seq, record.display()));
        }
        KnownTaskEventKind::BudgetUsage { item, delta } => {
            let entry = b.budget.entry(*item).or_insert((0, seq));
            entry.0 = entry.0.saturating_add(*delta);
            entry.1 = seq;
        }
        KnownTaskEventKind::CardShown { action, .. } => {
            b.questions.push((seq, action.clone()));
        }
        KnownTaskEventKind::CardAnswered { answer, .. } => {
            b.answers.push((seq, format!("{answer:?}")));
        }
        KnownTaskEventKind::QuestionShown { card, .. } => {
            b.questions.push((seq, card.clone()));
        }
        KnownTaskEventKind::QuestionAnswered { answer, .. } => {
            b.answers.push((seq, format!("{answer:?}")));
        }
        KnownTaskEventKind::ModelSwapped { from, to, .. } => {
            b.swaps.push((seq, from.clone(), to.clone()));
        }
        KnownTaskEventKind::TaskCreated { .. }
        | KnownTaskEventKind::DoneWhenChange { .. }
        | KnownTaskEventKind::DoneWhenAppliedByUser { .. }
        | KnownTaskEventKind::CriterionPassed { .. }
        | KnownTaskEventKind::CriterionFailed { .. }
        | KnownTaskEventKind::AcceptanceUnavailable
        | KnownTaskEventKind::ClaimChecked { .. }
        | KnownTaskEventKind::TaskCancelled { .. }
        | KnownTaskEventKind::TaskStopped
        | KnownTaskEventKind::Wake { .. }
        | KnownTaskEventKind::TurnStarted { .. }
        | KnownTaskEventKind::TurnEnded { .. }
        | KnownTaskEventKind::ContextBuilt { .. }
        | KnownTaskEventKind::ModelAsked { .. }
        | KnownTaskEventKind::ModelAnswer { .. }
        | KnownTaskEventKind::ModelUsage { .. }
        | KnownTaskEventKind::ToolNotRun { .. }
        | KnownTaskEventKind::ToolIntent { .. }
        | KnownTaskEventKind::GateResult { .. }
        | KnownTaskEventKind::ToolStarted { .. }
        | KnownTaskEventKind::ToolCompleted { .. }
        | KnownTaskEventKind::ToolFailed { .. }
        | KnownTaskEventKind::OutcomeUnknown { .. }
        | KnownTaskEventKind::Reconciled { .. }
        | KnownTaskEventKind::ActionVerified { .. }
        | KnownTaskEventKind::BudgetLimit { .. }
        | KnownTaskEventKind::DeviceReport { .. }
        | KnownTaskEventKind::DeliveryReport { .. }
        | KnownTaskEventKind::ModelUnreachable
        | KnownTaskEventKind::ComputerOffline
        | KnownTaskEventKind::GoalPaused { .. }
        | KnownTaskEventKind::Stalled { .. } => {}
    }
}
