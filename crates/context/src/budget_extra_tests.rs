use crate::budget_tests::assemble_input;
use crate::compile::compile;
use crate::compile_tests_support as sup;
use crate::input::StepRecord;
use crate::output::{Compiled, NewEvent};

#[test]
fn accumulate_collapses_after_fifteen_percent() {
    let usable: u64 = 32_000;
    let mut events: Vec<anybot_protocol::TaskEvent> = Vec::new();
    let mut prev_collapsed: Vec<String> = Vec::new();
    let mut collapse_points: Vec<(usize, u64)> = Vec::new();
    for index in 0..40usize {
        let steps: Vec<StepRecord> = (0..=index)
            .map(|i| sup::step_len(i, 700, &format!("r{i}")))
            .collect();
        let input = assemble_input(steps, usable, events.clone(), Vec::new());
        match compile(&input).unwrap() {
            Compiled::Sent { events: new, .. } => {
                let seq = u64::try_from(events.len()).unwrap_or(0).saturating_add(1);
                push_events(&mut events, &new, seq);
                let fresh: Vec<String> = events
                    .iter()
                    .rev()
                    .find_map(|e| anybot_domain::parse_collapsed(&e.kind).unwrap())
                    .map(|c| c.ids)
                    .unwrap_or_default();
                let added: usize = fresh
                    .iter()
                    .filter(|id| !prev_collapsed.contains(id))
                    .count();
                if added > 0 {
                    collapse_points.push((events.len(), u64::try_from(added).unwrap_or(0)));
                }
                prev_collapsed = fresh;
            }
            Compiled::Insufficient(_) => {}
        }
    }
    for (_at, added) in &collapse_points {
        let tokens = added.saturating_mul(700);
        let floor =
            crate::numbers::milli_of(usable, crate::numbers::K114_3_ACCUMULATE_MARGIN_MILLI);
        assert!(tokens >= floor, "collapse accumulation tracked");
    }
}

fn domain_reason(reason: crate::output::CollapseReason) -> anybot_domain::ContextCollapseReason {
    match reason {
        crate::output::CollapseReason::Accumulated => {
            anybot_domain::ContextCollapseReason::Accumulated
        }
        crate::output::CollapseReason::Budget => anybot_domain::ContextCollapseReason::Budget,
        crate::output::CollapseReason::Compact => anybot_domain::ContextCollapseReason::Compact,
        crate::output::CollapseReason::ModelSwapped => {
            anybot_domain::ContextCollapseReason::ModelSwapped
        }
        crate::output::CollapseReason::ReplaceAll => {
            anybot_domain::ContextCollapseReason::ReplaceAll
        }
    }
}

fn push_events(events: &mut Vec<anybot_protocol::TaskEvent>, new: &[NewEvent], start_seq: u64) {
    for (offset, e) in new.iter().enumerate() {
        let seq = start_seq.saturating_add(u64::try_from(offset).unwrap_or(0));
        match e {
            NewEvent::ContextBuilt => events.push(sup::known(
                seq,
                anybot_protocol::KnownTaskEventKind::ContextBuilt,
            )),
            NewEvent::MemoryDropped { ids } => {
                events.push(sup::event(seq, anybot_domain::memory_dropped_kind(ids)));
            }
            NewEvent::Collapsed { ids, reason } => events.push(sup::event(
                seq,
                anybot_domain::collapsed_kind(&anybot_domain::ContextCollapsed {
                    ids: ids.clone(),
                    reason: domain_reason(*reason),
                }),
            )),
        }
    }
}
