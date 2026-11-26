use anybot_protocol::{
    Criterion, DoneWhenChange, KnownTaskEventKind, Proposer, TaskDetail, TaskStatus,
};

use crate::task_view::{CriterionItem, PendingProposal, TaskView};

pub(crate) fn advance_budget(view: &mut TaskView, event: &KnownTaskEventKind) -> bool {
    match event {
        KnownTaskEventKind::BudgetUsage { item, delta } => {
            crate::task_budget::note_usage(view, *item, *delta);
            true
        }
        KnownTaskEventKind::BudgetLimit { item, limit } => {
            crate::task_budget::apply_limit(view, *item, *limit);
            true
        }
        KnownTaskEventKind::TaskCreated { .. }
        | KnownTaskEventKind::TaskCancelled { .. }
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
        | KnownTaskEventKind::CardShown { .. }
        | KnownTaskEventKind::CardAnswered { .. }
        | KnownTaskEventKind::DeviceReport { .. }
        | KnownTaskEventKind::DoneWhenChange { .. }
        | KnownTaskEventKind::DoneWhenAppliedByUser { .. } => false,
    }
}

pub(crate) fn advance_user_signals(view: &mut TaskView, event: &KnownTaskEventKind) -> bool {
    match event {
        KnownTaskEventKind::CardShown { action } => {
            view.status = TaskStatus::AwaitingUser;
            let action = action.clone();
            view.detail = Some(TaskDetail::ApprovalPending { action });
            true
        }
        KnownTaskEventKind::CardAnswered { answer } => {
            crate::task_engine_lifecycle::answer_card(view, *answer);
            true
        }
        KnownTaskEventKind::DeviceReport { kind, .. } => {
            crate::task_engine_lifecycle::report_device(view, *kind);
            true
        }
        KnownTaskEventKind::TaskCreated { .. }
        | KnownTaskEventKind::TaskCancelled { .. }
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
        | KnownTaskEventKind::DoneWhenChange { .. }
        | KnownTaskEventKind::DoneWhenAppliedByUser { .. } => false,
    }
}

pub(crate) fn advance_done_when(view: &mut TaskView, event: &KnownTaskEventKind) {
    match event {
        KnownTaskEventKind::DoneWhenChange {
            by,
            change,
            before,
            after,
            reason,
        } => {
            change_done_when(view, *by, change, before, after, reason);
        }
        KnownTaskEventKind::DoneWhenAppliedByUser { change } => {
            apply_pending(view, change);
        }
        KnownTaskEventKind::TaskCreated { .. }
        | KnownTaskEventKind::TaskCancelled { .. }
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
        | KnownTaskEventKind::DeviceReport { .. } => {}
    }
}

pub(crate) fn change_done_when(
    view: &mut TaskView,
    by: Proposer,
    change: &DoneWhenChange,
    before: &[Criterion],
    after: &[Criterion],
    reason: &str,
) {
    if by == Proposer::User || applies_directly(change) {
        apply_change(view, change);
        return;
    }
    let proposal = PendingProposal {
        by,
        change: change.clone(),
        before: before.to_vec(),
        after: after.to_vec(),
        reason: reason.to_owned(),
    };
    view.pending_proposals.push(proposal);
}

pub(crate) fn applies_directly(change: &DoneWhenChange) -> bool {
    matches!(
        change,
        DoneWhenChange::AddItem { .. } | DoneWhenChange::Tighten
    )
}

pub(crate) fn apply_pending(view: &mut TaskView, change: &DoneWhenChange) {
    let index = view
        .pending_proposals
        .iter()
        .position(|proposal| &proposal.change == change);
    if let Some(at) = index {
        view.pending_proposals.remove(at);
    }
    apply_change(view, change);
}

pub(crate) fn apply_change(view: &mut TaskView, change: &DoneWhenChange) {
    match change {
        DoneWhenChange::SetItems { items } => {
            view.criteria = items
                .iter()
                .map(|def| CriterionItem {
                    def: def.clone(),
                    passed: None,
                })
                .collect();
        }
        DoneWhenChange::AddItem { item } => {
            let item = CriterionItem {
                def: item.clone(),
                passed: None,
            };
            view.criteria.push(item);
        }
        DoneWhenChange::RemoveItem { index } => {
            if *index < view.criteria.len() {
                view.criteria.remove(*index);
            }
        }
        DoneWhenChange::ReplaceItem { index, item } => {
            if let Some(slot) = view.criteria.get_mut(*index) {
                slot.def = item.clone();
                slot.passed = None;
            }
        }
        DoneWhenChange::Tighten | DoneWhenChange::Loosen => {}
    }
}
