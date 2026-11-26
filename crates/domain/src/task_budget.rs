use std::collections::BTreeMap;

use anybot_foundation::TimestampMs;
use anybot_protocol::{BudgetItem, KnownTaskEventKind, TaskDetail, TaskStatus};

use crate::task_view::{BudgetOver, ROUND_ITEMS, TURN_STEP_GUARD, TaskView, UNLIMITED};

pub fn refresh_budget_status(view: &mut TaskView) {
    if let Some(over) = &view.budget.over {
        let over = over.clone();
        view.status = TaskStatus::Stuck;
        view.detail = Some(TaskDetail::OverBudget {
            item: over.item,
            used: over.used,
            limit: over.limit,
        });
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

#[must_use]
pub fn per_round(view: &TaskView, item: BudgetItem) -> bool {
    view.goal_task && ROUND_ITEMS.contains(&item) && !view.budget.explicit.contains(&item)
}

#[must_use]
pub fn compare_usage(view: &TaskView, item: BudgetItem) -> u64 {
    let raw = match item {
        BudgetItem::Elapsed => view
            .budget
            .used(item)
            .saturating_add(turn_active_ms(view, view.budget.last_ms) / 1_000),
        BudgetItem::Steps
        | BudgetItem::ToolCalls
        | BudgetItem::Turns
        | BudgetItem::Tokens
        | BudgetItem::Cost
        | BudgetItem::Retries => view.budget.used(item),
    };
    if per_round(view, item) {
        let baseline = view.budget.round_baseline.get(&item).copied().unwrap_or(0);
        raw.saturating_sub(baseline)
    } else {
        raw
    }
}

fn turn_active_ms(view: &TaskView, now: u64) -> u64 {
    let Some(start) = view.budget.turn_start_ms else {
        return 0;
    };
    let open_card = view
        .budget
        .card_start_ms
        .map_or(0, |c| now.saturating_sub(c));
    now.saturating_sub(start)
        .saturating_sub(view.budget.turn_paused_ms)
        .saturating_sub(open_card)
}

fn over_at_cap(view: &TaskView, item: BudgetItem) -> Option<BudgetOver> {
    let limit = view.budget.limits.get(&item).copied()?;
    let used = compare_usage(view, item);
    if used >= limit {
        return Some(BudgetOver { item, used, limit });
    }
    None
}

fn over_past_limit(view: &TaskView, item: BudgetItem) -> Option<BudgetOver> {
    let limit = view.budget.limits.get(&item).copied()?;
    let used = compare_usage(view, item);
    if used > limit {
        return Some(BudgetOver { item, used, limit });
    }
    None
}

fn steps_guard(view: &TaskView) -> Option<BudgetOver> {
    if view.budget.turn_steps >= TURN_STEP_GUARD {
        return Some(BudgetOver {
            item: BudgetItem::Steps,
            used: TURN_STEP_GUARD,
            limit: TURN_STEP_GUARD,
        });
    }
    None
}

pub fn mark_over(view: &mut TaskView, over: Option<BudgetOver>) {
    if view.budget.over.is_none() {
        view.budget.over = over;
    }
    refresh_budget_status(view);
}

pub fn note_ask_attempt(view: &mut TaskView) {
    if view.budget.over.is_some() {
        refresh_budget_status(view);
        return;
    }
    let over = steps_guard(view)
        .or_else(|| over_at_cap(view, BudgetItem::Steps))
        .or_else(|| over_at_cap(view, BudgetItem::Elapsed))
        .or_else(|| over_at_cap(view, BudgetItem::Tokens))
        .or_else(|| over_at_cap(view, BudgetItem::Cost));
    mark_over(view, over);
}

pub fn note_action_attempt(view: &mut TaskView) {
    if view.budget.over.is_some() {
        refresh_budget_status(view);
        return;
    }
    let over = over_at_cap(view, BudgetItem::ToolCalls)
        .or_else(|| over_at_cap(view, BudgetItem::Elapsed))
        .or_else(|| over_at_cap(view, BudgetItem::Tokens))
        .or_else(|| over_at_cap(view, BudgetItem::Cost));
    mark_over(view, over);
}

pub fn note_turn_attempt(view: &mut TaskView) {
    if view.budget.over.is_some() {
        refresh_budget_status(view);
        return;
    }
    let over = over_at_cap(view, BudgetItem::Turns);
    mark_over(view, over);
}

pub fn note_usage(view: &mut TaskView, item: BudgetItem, delta: u64) {
    if item == BudgetItem::Steps {
        note_ask_attempt(view);
        if view.budget.over.is_none() {
            view.budget.add_usage(item, delta);
            view.budget.turn_steps = view.budget.turn_steps.saturating_add(delta);
        }
        refresh_budget_status(view);
        return;
    }
    view.budget.add_usage(item, delta);
    if !matches!(item, BudgetItem::Retries) {
        let over = over_past_limit(view, item);
        mark_over(view, over);
    }
    refresh_budget_status(view);
}

pub fn apply_limit(view: &mut TaskView, item: BudgetItem, limit: u64) {
    if limit == UNLIMITED {
        view.budget.limits.remove(&item);
        view.budget.explicit.remove(&item);
    } else {
        view.budget.limits.insert(item, limit);
        view.budget.explicit.insert(item);
    }
    clear_over_if_raised(view, item, limit);
    refresh_budget_status(view);
}

fn clear_over_if_raised(view: &mut TaskView, item: BudgetItem, limit: u64) {
    let cleared = view
        .budget
        .over
        .as_ref()
        .is_some_and(|over| over.item == item && (limit == UNLIMITED || over.used < limit));
    if cleared {
        view.budget.over = None;
    }
}

pub fn begin_turn(view: &mut TaskView) {
    view.gate.step_rejected = false;
    note_turn_attempt(view);
    if view.budget.over.is_none() {
        view.budget.add_usage(BudgetItem::Turns, 1);
    }
    reset_round_baseline(view);
    view.evidence.turn_reply = false;
    refresh_budget_status(view);
}

fn reset_round_baseline(view: &mut TaskView) {
    view.budget.turn_steps = 0;
    let mut baseline = BTreeMap::new();
    for item in ROUND_ITEMS {
        baseline.insert(item, view.budget.used(item));
    }
    view.budget.round_baseline = baseline;
}

pub fn note_time(view: &mut TaskView, at: TimestampMs, kind: &KnownTaskEventKind) {
    let millis = at.millis();
    match kind {
        KnownTaskEventKind::TurnStarted { .. } => {
            view.budget.last_ms = millis;
            view.budget.turn_start_ms = Some(millis);
            view.budget.turn_paused_ms = 0;
            view.budget.card_start_ms = None;
        }
        KnownTaskEventKind::CardShown { .. } => {
            view.budget.last_ms = millis;
            view.budget.card_start_ms = Some(millis);
        }
        KnownTaskEventKind::CardAnswered { .. } => {
            view.budget.last_ms = millis;
            pause_answered(view, millis);
        }
        KnownTaskEventKind::TurnEnded { .. } => {
            view.budget.last_ms = millis;
            turn_ended(view, millis);
        }
        KnownTaskEventKind::BudgetUsage { .. }
        | KnownTaskEventKind::BudgetLimit { .. }
        | KnownTaskEventKind::ToolIntent { .. }
        | KnownTaskEventKind::ToolStarted { .. } => {
            view.budget.last_ms = millis;
        }
        KnownTaskEventKind::TaskCreated { .. }
        | KnownTaskEventKind::TaskCancelled { .. }
        | KnownTaskEventKind::TaskStopped
        | KnownTaskEventKind::Wake { .. }
        | KnownTaskEventKind::GoalPaused { .. }
        | KnownTaskEventKind::ComputerOffline
        | KnownTaskEventKind::ModelUnreachable
        | KnownTaskEventKind::Stalled { .. }
        | KnownTaskEventKind::ContextBuilt { .. }
        | KnownTaskEventKind::ModelAsked { .. }
        | KnownTaskEventKind::ModelAnswer { .. }
        | KnownTaskEventKind::ModelUsage { .. }
        | KnownTaskEventKind::ToolNotRun { .. }
        | KnownTaskEventKind::ModelSwapped { .. }
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
        | KnownTaskEventKind::DeviceReport { .. }
        | KnownTaskEventKind::DoneWhenChange { .. }
        | KnownTaskEventKind::DoneWhenAppliedByUser { .. } => {}
    }
}

fn pause_answered(view: &mut TaskView, at: u64) {
    if let Some(start) = view.budget.card_start_ms {
        let delta = at.saturating_sub(start);
        view.budget.turn_paused_ms = view.budget.turn_paused_ms.saturating_add(delta);
        view.budget.card_start_ms = None;
    }
}

fn turn_ended(view: &mut TaskView, at: u64) {
    if view.budget.turn_start_ms.is_none() {
        return;
    }
    let active_secs = turn_active_ms(view, at) / 1_000;
    view.budget.turn_start_ms = None;
    view.budget.card_start_ms = None;
    view.budget.turn_paused_ms = 0;
    if active_secs > 0 {
        note_usage(view, BudgetItem::Elapsed, active_secs);
    }
}

#[cfg(test)]
#[path = "task_budget_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "task_budget_prop_tests.rs"]
mod prop_tests;

#[cfg(test)]
#[path = "task_budget_fixtures.rs"]
mod fixtures;

#[cfg(test)]
#[path = "task_budget_live_tests.rs"]
mod live_tests;
