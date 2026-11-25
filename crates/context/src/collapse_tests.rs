use std::collections::BTreeSet;

use super::{StepView, budget_collapse, plan_collapse, recorded_collapsed};

fn view(index: usize, ref_id: &str, tokens_full: u64) -> StepView {
    StepView {
        index,
        source_ref: ref_id.to_owned(),
        keep_full: false,
        short: false,
        tokens_full,
        tokens_masked: 1,
    }
}

#[test]
fn test_collapse_accumulate_over_margin() {
    let views: Vec<StepView> = (0..12).map(|i| view(i, &format!("r{i}"), 100)).collect();
    let recorded = BTreeSet::new();
    let (collapsed, events) = plan_collapse(&views, &recorded, 1000, false, false);
    assert_eq!(collapsed.len(), 2);
    assert_eq!(events.len(), 1);
}

#[test]
fn test_collapse_stays_under_margin() {
    let views: Vec<StepView> = (0..12).map(|i| view(i, &format!("r{i}"), 10)).collect();
    let recorded = BTreeSet::new();
    let (collapsed, events) = plan_collapse(&views, &recorded, 1000, false, false);
    assert!(collapsed.is_empty());
    assert!(events.is_empty());
}

#[test]
fn test_collapse_compact_collapses_all_old() {
    let views: Vec<StepView> = (0..30).map(|i| view(i, &format!("r{i}"), 5)).collect();
    let recorded = BTreeSet::new();
    let (collapsed, events) = plan_collapse(&views, &recorded, 1000, false, true);
    assert_eq!(collapsed.len(), 20);
    assert_eq!(events.len(), 1);
}

#[test]
fn test_collapse_budget_stage_keeps_last_two() {
    let views: Vec<StepView> = (0..6).map(|i| view(i, &format!("r{i}"), 50)).collect();
    let recorded = BTreeSet::new();
    let ids = budget_collapse(&views, &recorded, 2);
    assert_eq!(ids.len(), 4);
}

#[test]
fn test_collapse_recorded_ids_are_stable_between_events() {
    let views: Vec<StepView> = (0..12).map(|i| view(i, &format!("r{i}"), 100)).collect();
    let recorded: BTreeSet<String> = views.iter().take(1).map(|v| v.source_ref.clone()).collect();
    let (collapsed, events) = plan_collapse(&views, &recorded, 600, false, false);
    for id in &recorded {
        assert!(collapsed.contains(id));
    }
    let added = events.first().expect("collapse event");
    match added {
        super::NewEvent::Collapsed { ids, .. } => {
            for id in ids {
                assert!(!recorded.contains(id));
            }
        }
        super::NewEvent::ContextBuilt => {}
    }
}

#[test]
fn test_collapse_short_and_protected_are_never_collapsed() {
    let mut views: Vec<StepView> = (0..12).map(|i| view(i, &format!("r{i}"), 50)).collect();
    if let Some(v) = views.get_mut(0) {
        v.short = true;
    }
    if let Some(v) = views.get_mut(1) {
        v.keep_full = true;
    }
    let recorded = BTreeSet::new();
    let (collapsed, _) = plan_collapse(&views, &recorded, 1000, false, true);
    assert!(!collapsed.contains("r0"));
    assert!(!collapsed.contains("r1"));
}

#[test]
fn test_collapse_recorded_scan_ignores_other_events() {
    use anybot_foundation::{EventId, SystemClock, TimestampMs};
    use anybot_protocol::{KnownTaskEventKind, TaskEvent};
    let e = TaskEvent {
        event_id: EventId::generate(&SystemClock),
        seq: 1,
        at_ms: TimestampMs::from_millis(1),
        kind: anybot_protocol::TaskEventKind::Known(KnownTaskEventKind::ContextBuilt),
    };
    assert!(recorded_collapsed(&[e]).is_empty());
}
