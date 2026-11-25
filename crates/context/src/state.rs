use std::collections::BTreeMap;

use anybot_foundation::CallId;
use anybot_protocol::{
    BudgetItem, Criterion, DoneWhenChange, KnownTaskEventKind, TaskEvent, TaskEventKind, ToolEffect,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepStateKind {
    Open,
    Done,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepState {
    pub tool: String,
    pub intent_seq: u64,
    pub state: StepStateKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CriterionState {
    pub text: String,
    pub declared_seq: u64,
    pub outcome: Option<(bool, u64)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StateBuilder {
    pub goal: Option<(String, u64)>,
    pub items: Vec<Criterion>,
    pub criteria: Vec<CriterionState>,
    pub steps: BTreeMap<String, StepState>,
    pub evidence: Vec<(u64, String)>,
    pub questions: Vec<(u64, String)>,
    pub answers: Vec<(u64, String)>,
    pub pending: BTreeMap<String, u64>,
    pub unknowns: BTreeMap<String, u64>,
    pub budget: BTreeMap<BudgetItem, (u64, u64)>,
    pub swaps: Vec<(u64, String, String)>,
}

#[must_use]
pub fn build_state(events: &[TaskEvent]) -> StateBuilder {
    let mut sorted: Vec<&TaskEvent> = events.iter().collect();
    sorted.sort_by_key(|e| e.seq);
    let mut b = StateBuilder::default();
    for e in sorted {
        apply(&mut b, e.seq, &e.kind);
    }
    b
}

fn apply(b: &mut StateBuilder, seq: u64, kind: &TaskEventKind) {
    if let TaskEventKind::Known(known) = kind {
        apply_known(b, seq, known);
    }
}

pub(crate) fn criterion_text(criterion: &Criterion) -> String {
    format!("{criterion:?}")
}

fn set_criteria(b: &mut StateBuilder, seq: u64, items: &[Criterion]) {
    b.items = items.to_vec();
    b.criteria = items
        .iter()
        .map(|c| CriterionState {
            text: criterion_text(c),
            declared_seq: seq,
            outcome: None,
        })
        .collect();
}

fn apply_change(b: &mut StateBuilder, seq: u64, change: &DoneWhenChange) {
    let mut items = b.items.clone();
    apply_change_to(&mut items, change);
    set_criteria(b, seq, &items);
}

fn apply_change_to(items: &mut Vec<Criterion>, change: &DoneWhenChange) {
    match change {
        DoneWhenChange::SetItems { items: new } => {
            items.clone_from(new);
        }
        DoneWhenChange::AddItem { item } => items.push(item.clone()),
        DoneWhenChange::RemoveItem { index } => {
            let kept: Vec<Criterion> = items
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != *index)
                .map(|(_, c)| c.clone())
                .collect();
            *items = kept;
        }
        DoneWhenChange::ReplaceItem { index, item } => {
            if let Some(slot) = items.get_mut(*index) {
                slot.clone_from(item);
            }
        }
        DoneWhenChange::Tighten | DoneWhenChange::Loosen => {}
    }
}

fn apply_known(b: &mut StateBuilder, seq: u64, known: &KnownTaskEventKind) {
    apply_criteria(b, seq, known);
    apply_steps(b, seq, known);
    apply_notes(b, seq, known);
}

fn apply_criteria(b: &mut StateBuilder, seq: u64, known: &KnownTaskEventKind) {
    match known {
        KnownTaskEventKind::TaskCreated { goal } => {
            b.goal = Some((goal.clone(), seq));
        }
        KnownTaskEventKind::DoneWhenChange { after, .. } => {
            set_criteria(b, seq, after);
        }
        KnownTaskEventKind::DoneWhenAppliedByUser { change } => {
            apply_change(b, seq, change);
        }
        KnownTaskEventKind::CriterionPassed { index, .. } => {
            set_outcome(b, *index, true, seq);
        }
        KnownTaskEventKind::CriterionFailed { index, .. } => {
            set_outcome(b, *index, false, seq);
        }
        KnownTaskEventKind::AcceptanceUnavailable
        | KnownTaskEventKind::ClaimChecked { .. }
        | KnownTaskEventKind::TaskCancelled { .. }
        | KnownTaskEventKind::TaskStopped
        | KnownTaskEventKind::Wake { .. }
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
        | KnownTaskEventKind::BudgetUsage { .. }
        | KnownTaskEventKind::BudgetLimit { .. }
        | KnownTaskEventKind::CardShown { .. }
        | KnownTaskEventKind::CardAnswered { .. }
        | KnownTaskEventKind::DeviceReport { .. }
        | KnownTaskEventKind::ModelUnreachable
        | KnownTaskEventKind::ComputerOffline
        | KnownTaskEventKind::GoalPaused { .. }
        | KnownTaskEventKind::Stalled { .. } => {}
    }
}

fn apply_steps(b: &mut StateBuilder, seq: u64, known: &KnownTaskEventKind) {
    match known {
        KnownTaskEventKind::ToolIntent {
            call_id,
            name,
            level,
            ..
        } => record_intent(b, seq, call_id, name, *level),
        KnownTaskEventKind::ToolCompleted { call_id } => {
            update_step(b, call_id, StepStateKind::Done, seq);
        }
        KnownTaskEventKind::ToolFailed { call_id, .. } => {
            update_step(b, call_id, StepStateKind::Failed, seq);
        }
        KnownTaskEventKind::OutcomeUnknown { call_id } => {
            b.unknowns.insert(call_id.as_str().clone(), seq);
        }
        KnownTaskEventKind::Reconciled { call_id, .. } => {
            b.unknowns.remove(&call_id.as_str());
        }
        KnownTaskEventKind::ActionVerified { call_id, .. } => {
            b.pending.remove(&call_id.as_str());
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
        | KnownTaskEventKind::TurnStarted
        | KnownTaskEventKind::TurnEnded { .. }
        | KnownTaskEventKind::TurnText { .. }
        | KnownTaskEventKind::ContextBuilt
        | KnownTaskEventKind::ToolStarted { .. }
        | KnownTaskEventKind::EvidenceAdded { .. }
        | KnownTaskEventKind::BudgetUsage { .. }
        | KnownTaskEventKind::BudgetLimit { .. }
        | KnownTaskEventKind::CardShown { .. }
        | KnownTaskEventKind::CardAnswered { .. }
        | KnownTaskEventKind::ModelSwapped { .. }
        | KnownTaskEventKind::DeviceReport { .. }
        | KnownTaskEventKind::ModelUnreachable
        | KnownTaskEventKind::ComputerOffline
        | KnownTaskEventKind::GoalPaused { .. }
        | KnownTaskEventKind::Stalled { .. } => {}
    }
}

fn record_intent(b: &mut StateBuilder, seq: u64, call_id: &CallId, name: &str, level: ToolEffect) {
    b.steps.insert(
        call_id.as_str().clone(),
        StepState {
            tool: name.to_owned(),
            intent_seq: seq,
            state: StepStateKind::Open,
        },
    );
    if level != ToolEffect::ReadOnly {
        b.pending.insert(call_id.as_str().clone(), seq);
    }
}

fn apply_notes(b: &mut StateBuilder, seq: u64, known: &KnownTaskEventKind) {
    match known {
        KnownTaskEventKind::EvidenceAdded { summary } => {
            b.evidence.push((seq, summary.clone()));
        }
        KnownTaskEventKind::BudgetUsage { item, delta } => {
            let entry = b.budget.entry(*item).or_insert((0, seq));
            entry.0 = entry.0.saturating_add(*delta);
            entry.1 = seq;
        }
        KnownTaskEventKind::CardShown { action } => {
            b.questions.push((seq, action.clone()));
        }
        KnownTaskEventKind::CardAnswered { answer } => {
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
        | KnownTaskEventKind::TurnStarted
        | KnownTaskEventKind::TurnEnded { .. }
        | KnownTaskEventKind::TurnText { .. }
        | KnownTaskEventKind::ContextBuilt
        | KnownTaskEventKind::ToolIntent { .. }
        | KnownTaskEventKind::ToolStarted { .. }
        | KnownTaskEventKind::ToolCompleted { .. }
        | KnownTaskEventKind::ToolFailed { .. }
        | KnownTaskEventKind::OutcomeUnknown { .. }
        | KnownTaskEventKind::Reconciled { .. }
        | KnownTaskEventKind::ActionVerified { .. }
        | KnownTaskEventKind::BudgetLimit { .. }
        | KnownTaskEventKind::DeviceReport { .. }
        | KnownTaskEventKind::ModelUnreachable
        | KnownTaskEventKind::ComputerOffline
        | KnownTaskEventKind::GoalPaused { .. }
        | KnownTaskEventKind::Stalled { .. } => {}
    }
}

fn set_outcome(b: &mut StateBuilder, index: usize, passed: bool, seq: u64) {
    if let Some(c) = b.criteria.get_mut(index) {
        c.outcome = Some((passed, seq));
    }
}

fn update_step(b: &mut StateBuilder, call_id: &CallId, state: StepStateKind, seq: u64) {
    if let Some(s) = b.steps.get_mut(&call_id.as_str()) {
        s.state = state;
        s.intent_seq = seq;
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
