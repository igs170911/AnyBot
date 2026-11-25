use std::collections::BTreeSet;

use anybot_domain::{ContextCollapseReason, parse_collapsed};

use crate::input::StepRecord;
use crate::numbers::{
    K114_1_RECENT_STEPS, K114_3_ACCUMULATE_MARGIN_MILLI, K114_3_SHORT_RESULT_CHARS, milli_of,
};
use crate::output::{CollapseReason, NewEvent};

#[derive(Debug, Clone)]
pub struct StepView {
    pub index: usize,
    pub source_ref: String,
    pub keep_full: bool,
    pub short: bool,
    pub tokens_full: u64,
    pub tokens_masked: u64,
}

#[must_use]
pub fn step_view(
    step: &StepRecord,
    index: usize,
    tokens_full: u64,
    tokens_masked: u64,
) -> StepView {
    StepView {
        index,
        source_ref: step.source_ref.clone(),
        keep_full: step.keeps_full(),
        short: !step.has_output() || step.content.chars().count() <= K114_3_SHORT_RESULT_CHARS,
        tokens_full,
        tokens_masked,
    }
}

#[must_use]
pub fn recorded_collapsed(events: &[anybot_protocol::TaskEvent]) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for e in events {
        if let Some(c) = parse_collapsed(&e.kind) {
            ids.extend(c.ids);
        }
    }
    ids
}

#[must_use]
pub fn model_swapped_after_last_built(events: &[anybot_protocol::TaskEvent]) -> bool {
    let mut last_built = 0u64;
    let mut swapped = false;
    for e in events {
        if matches!(
            &e.kind,
            anybot_protocol::TaskEventKind::Known(
                anybot_protocol::KnownTaskEventKind::ContextBuilt
            )
        ) {
            last_built = e.seq;
        }
        if matches!(
            &e.kind,
            anybot_protocol::TaskEventKind::Known(
                anybot_protocol::KnownTaskEventKind::ModelSwapped { .. }
            )
        ) && e.seq > last_built
        {
            swapped = true;
        }
    }
    swapped
}

#[must_use]
pub fn plan_collapse(
    views: &[StepView],
    recorded: &BTreeSet<String>,
    usable: u64,
    swap: bool,
    compact: bool,
) -> (BTreeSet<String>, Vec<NewEvent>) {
    let margin = milli_of(usable, K114_3_ACCUMULATE_MARGIN_MILLI);
    let old = old_candidates(views);
    let fresh: Vec<&StepView> = old
        .into_iter()
        .filter(|v| !recorded.contains(&v.source_ref))
        .collect();
    let total: u64 = fresh.iter().map(|v| v.tokens_full).sum();
    let target: BTreeSet<String> = if compact || total > margin {
        fresh.iter().map(|v| v.source_ref.clone()).collect()
    } else {
        BTreeSet::new()
    };
    let mut collapsed: BTreeSet<String> = recorded.clone();
    if swap && !compact {
        collapsed.clear();
    }
    collapsed.extend(target.iter().cloned());
    let mut events = Vec::new();
    let added: Vec<String> = target
        .iter()
        .filter(|id| !recorded.contains(*id))
        .cloned()
        .collect();
    if !added.is_empty() {
        let reason = collapse_reason(compact, swap);
        events.push(NewEvent::Collapsed { ids: added, reason });
    }
    (collapsed, events)
}

fn collapse_reason(compact: bool, swap: bool) -> CollapseReason {
    if compact {
        return CollapseReason::Compact;
    }
    if swap {
        return CollapseReason::ModelSwapped;
    }
    CollapseReason::Accumulated
}

fn old_candidates(views: &[StepView]) -> Vec<&StepView> {
    let cutoff = views.len().saturating_sub(K114_1_RECENT_STEPS);
    views
        .iter()
        .filter(|v| v.index < cutoff && !v.short && !v.keep_full)
        .collect()
}

#[must_use]
pub fn budget_collapse(
    views: &[StepView],
    collapsed: &BTreeSet<String>,
    keep_recent: usize,
) -> Vec<String> {
    let cutoff = views.len().saturating_sub(keep_recent);
    views
        .iter()
        .filter(|v| v.index < cutoff && !v.short && !v.keep_full)
        .map(|v| v.source_ref.clone())
        .filter(|id| !collapsed.contains(id))
        .collect()
}

#[must_use]
pub fn reason_to_domain(reason: CollapseReason) -> ContextCollapseReason {
    match reason {
        CollapseReason::Accumulated => ContextCollapseReason::Accumulated,
        CollapseReason::Budget => ContextCollapseReason::Budget,
        CollapseReason::Compact => ContextCollapseReason::Compact,
        CollapseReason::ModelSwapped => ContextCollapseReason::ModelSwapped,
    }
}

#[cfg(test)]
#[path = "collapse_tests.rs"]
mod tests;
