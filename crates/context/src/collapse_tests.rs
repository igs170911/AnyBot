use std::collections::BTreeSet;

use super::{
    StepView, budget_collapse, model_swapped_after_last_built, plan_collapse, recorded_collapsed,
};

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
        super::NewEvent::ContextBuilt | super::NewEvent::MemoryDropped { .. } => {}
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
    assert!(recorded_collapsed(&[e]).unwrap().is_empty());
}

#[test]
fn swap_flag_clears_after_next_build() {
    let events = vec![
        crate::fixtures::event_with_seq(
            1,
            anybot_protocol::TaskEventKind::Known(
                anybot_protocol::KnownTaskEventKind::ContextBuilt,
            ),
        ),
        crate::fixtures::event_with_seq(
            2,
            anybot_protocol::TaskEventKind::Known(
                anybot_protocol::KnownTaskEventKind::ModelSwapped {
                    from: "a".to_owned(),
                    to: "b".to_owned(),
                    reason: "r".to_owned(),
                },
            ),
        ),
        crate::fixtures::event_with_seq(
            3,
            anybot_protocol::TaskEventKind::Known(
                anybot_protocol::KnownTaskEventKind::ContextBuilt,
            ),
        ),
        crate::fixtures::event_with_seq(
            4,
            anybot_protocol::TaskEventKind::Known(
                anybot_protocol::KnownTaskEventKind::ContextBuilt,
            ),
        ),
    ];
    assert!(!model_swapped_after_last_built(&events));
}

#[test]
fn replace_all_event_replaces_recorded_ids() {
    let old = anybot_domain::collapsed_kind(&anybot_domain::ContextCollapsed {
        ids: vec!["r1".to_owned(), "r2".to_owned()],
        reason: anybot_domain::ContextCollapseReason::Accumulated,
    });
    let fresh = anybot_domain::collapsed_kind(&anybot_domain::ContextCollapsed {
        ids: vec!["r3".to_owned()],
        reason: anybot_domain::ContextCollapseReason::ReplaceAll,
    });
    let events = vec![
        crate::fixtures::event_with_seq(1, old),
        crate::fixtures::event_with_seq(2, fresh),
    ];
    let recorded = recorded_collapsed(&events).unwrap();
    assert_eq!(recorded, BTreeSet::from(["r3".to_owned()]));
}

#[test]
fn smaller_model_swap_recollapses_all_old() {
    let views: Vec<StepView> = (0..30).map(|i| view(i, &format!("r{i}"), 375)).collect();
    let recorded: BTreeSet<String> = (0..20).map(|i| format!("r{i}")).collect();
    let (collapsed, events) = plan_collapse(&views, &recorded, 24_000, true, false);
    assert_eq!(collapsed.len(), 20);
    assert_eq!(events.len(), 1);
    assert!(matches!(
        events.first().expect("event"),
        super::NewEvent::Collapsed {
            reason: super::CollapseReason::ReplaceAll,
            ..
        }
    ));
}

#[test]
fn larger_model_swap_restores_everything() {
    let views: Vec<StepView> = (0..30).map(|i| view(i, &format!("r{i}"), 375)).collect();
    let recorded: BTreeSet<String> = (0..20).map(|i| format!("r{i}")).collect();
    let (collapsed, events) = plan_collapse(&views, &recorded, 128_000, true, false);
    assert!(collapsed.is_empty());
    assert_eq!(events.len(), 1);
}
