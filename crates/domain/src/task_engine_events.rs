use anybot_protocol::{BudgetItem, KnownTaskEventKind, ReconcileOutcome};

use crate::task_view::TaskView;

pub(crate) fn advance_action(view: &mut TaskView, event: &KnownTaskEventKind) -> bool {
    if let KnownTaskEventKind::ToolFailed {
        call_id,
        name,
        args_summary,
        ..
    } = event
    {
        step_failed(view, name, args_summary);
        note_write(view, &call_id.as_str(), false);
        return true;
    }
    if let KnownTaskEventKind::ToolCompleted { call_id, .. } = event {
        note_write(view, &call_id.as_str(), true);
        return true;
    }
    if let KnownTaskEventKind::GateResult { call_id, gate } = event {
        gate_write(view, call_id, gate);
        return true;
    }
    action_core(view, event)
}

fn gate_write(
    view: &mut TaskView,
    call_id: &anybot_foundation::CallId,
    gate: &anybot_protocol::GateRecord,
) {
    if !gate.read_only {
        view.write_calls.insert(call_id.as_str());
    }
}

pub(crate) fn note_write(view: &mut TaskView, call_id: &str, ok: bool) {
    if view.write_calls.contains(call_id) {
        view.gate.last_write_ok = Some(ok);
    }
}

fn action_core(view: &mut TaskView, event: &KnownTaskEventKind) -> bool {
    match event {
        KnownTaskEventKind::ToolIntent { call_id, level, .. } => {
            crate::task_budget::note_action_attempt(view);
            view.action_levels.insert(call_id.as_str(), *level);
            true
        }
        KnownTaskEventKind::ToolStarted { call_id } => {
            started(view, call_id);
            true
        }
        KnownTaskEventKind::OutcomeUnknown { call_id } => {
            view.unresolved_unknowns.insert(call_id.as_str());
            true
        }
        KnownTaskEventKind::Reconciled { call_id, outcome } => {
            reconcile(view, call_id, *outcome);
            true
        }
        KnownTaskEventKind::ActionVerified { call_id, .. } => {
            view.verified_actions.insert(call_id.as_str());
            true
        }
        KnownTaskEventKind::ToolCompleted { .. }
        | KnownTaskEventKind::ToolFailed { .. }
        | KnownTaskEventKind::ToolNotRun { .. }
        | KnownTaskEventKind::EvidenceAdded { .. }
        | KnownTaskEventKind::GateResult { .. } => true,
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
        | KnownTaskEventKind::ModelSwapped { .. }
        | KnownTaskEventKind::CriterionPassed { .. }
        | KnownTaskEventKind::CriterionFailed { .. }
        | KnownTaskEventKind::AcceptanceUnavailable
        | KnownTaskEventKind::VerificationStarted
        | KnownTaskEventKind::ClaimChecked { .. }
        | KnownTaskEventKind::BudgetUsage { .. }
        | KnownTaskEventKind::BudgetLimit { .. }
        | KnownTaskEventKind::CardShown { .. }
        | KnownTaskEventKind::CardAnswered { .. }
        | KnownTaskEventKind::DeviceReport { .. }
        | KnownTaskEventKind::DeliveryReport { .. }
        | KnownTaskEventKind::DoneWhenChange { .. }
        | KnownTaskEventKind::DoneWhenAppliedByUser { .. } => false,
    }
}

pub(crate) fn advance_criteria(view: &mut TaskView, event: &KnownTaskEventKind) -> bool {
    match event {
        KnownTaskEventKind::CriterionPassed { index, .. } => {
            apply_criterion_result(view, *index, Some(true));
            finish_if_verifying_done(view);
            true
        }
        KnownTaskEventKind::CriterionFailed { index, .. } => {
            apply_criterion_result(view, *index, Some(false));
            finish_if_verifying_done(view);
            true
        }
        KnownTaskEventKind::AcceptanceUnavailable => {
            view.gate.acceptance_unavailable = true;
            finish_if_verifying_done(view);
            true
        }
        KnownTaskEventKind::ClaimChecked {
            covers_claim,
            verified,
        } => {
            if !covers_claim || !verified {
                view.evidence.claims_ok = false;
            }
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
        | KnownTaskEventKind::VerificationStarted
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
        | KnownTaskEventKind::BudgetUsage { .. }
        | KnownTaskEventKind::BudgetLimit { .. }
        | KnownTaskEventKind::CardShown { .. }
        | KnownTaskEventKind::CardAnswered { .. }
        | KnownTaskEventKind::DeviceReport { .. }
        | KnownTaskEventKind::DeliveryReport { .. }
        | KnownTaskEventKind::DoneWhenChange { .. }
        | KnownTaskEventKind::DoneWhenAppliedByUser { .. } => false,
    }
}

pub(crate) fn started(view: &mut TaskView, call_id: &anybot_foundation::CallId) {
    crate::task_lifecycle_ops::wake(view);
    crate::task_budget::note_action_attempt(view);
    if view.budget.over.is_none() {
        view.budget.add_usage(BudgetItem::ToolCalls, 1);
    }
    view.evidence.actions_taken = true;
    view.note_action(&call_id.as_str());
}

fn reconcile(view: &mut TaskView, call_id: &anybot_foundation::CallId, outcome: ReconcileOutcome) {
    let key = call_id.as_str();
    if outcome == ReconcileOutcome::Unknown {
        view.unresolved_unknowns.insert(key);
    } else {
        view.unresolved_unknowns.remove(&key);
    }
}

pub(crate) fn step_failed(view: &mut TaskView, name: &str, args_summary: &str) {
    let key = format!("{name}:{args_summary}");
    let next = view
        .budget
        .step_fails
        .get(&key)
        .copied()
        .unwrap_or(0u64)
        .saturating_add(1);
    view.budget.step_fails.insert(key, next);
    if next >= view.budget.step_fail_limit() {
        view.gate.step_failed = true;
    }
}

pub(crate) fn apply_criterion_result(view: &mut TaskView, index: usize, passed: Option<bool>) {
    if let Some(item) = view.criteria.get_mut(index) {
        item.passed = passed;
    }
}

fn finish_if_verifying_done(view: &mut TaskView) {
    if !matches!(view.detail, Some(anybot_protocol::TaskDetail::Verifying)) {
        return;
    }
    if view.gate.acceptance_unavailable
        || (!view.criteria.is_empty() && view.criteria.iter().all(|item| item.passed.is_some()))
    {
        crate::task_gate::finish_turn(view);
    }
}
