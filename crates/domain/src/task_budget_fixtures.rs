use anybot_foundation::{CallId, SystemClock};
use anybot_protocol::{BudgetItem, KnownTaskEventKind, ToolEffect};

use crate::task_engine::advance;
use crate::task_view::{TaskView, initial_view};

pub(crate) fn drive(kinds: &[KnownTaskEventKind]) -> TaskView {
    let mut view = initial_view();
    for kind in kinds {
        advance(&mut view, kind);
    }
    view
}

pub(crate) fn call() -> CallId {
    CallId::generate(&SystemClock)
}

pub(crate) fn created() -> KnownTaskEventKind {
    KnownTaskEventKind::TaskCreated {
        goal: "g".to_owned(),
        opened_by: anybot_protocol::TaskOpener::You,
        bot: anybot_foundation::BotId::generate(&SystemClock),
        parent: None,
    }
}

pub(crate) fn turns_ended() -> KnownTaskEventKind {
    KnownTaskEventKind::TurnEnded {
        reason: anybot_protocol::TurnEndReason::Answered,
        reply_delivered: true,
    }
}

pub(crate) fn asks(n: usize) -> Vec<KnownTaskEventKind> {
    (0..n)
        .map(|_| KnownTaskEventKind::BudgetUsage {
            item: BudgetItem::Steps,
            delta: 1,
        })
        .collect()
}

pub(crate) fn intent() -> KnownTaskEventKind {
    KnownTaskEventKind::ToolIntent {
        call_id: call(),
        name: "n".to_owned(),
        args_summary: "a".to_owned(),
        level: ToolEffect::ReadOnly,
    }
}

pub(crate) fn pair(n: usize) -> Vec<KnownTaskEventKind> {
    (0..n)
        .flat_map(|_| {
            vec![
                intent(),
                KnownTaskEventKind::ToolStarted { call_id: call() },
            ]
        })
        .collect()
}

pub(crate) fn ev_at(seq: u64, at: u64, kind: KnownTaskEventKind) -> anybot_protocol::TaskEvent {
    use anybot_foundation::{EventId, TimestampMs};
    use anybot_protocol::TaskEventKind;
    anybot_protocol::TaskEvent {
        event_id: EventId::generate(&SystemClock),
        seq,
        at_ms: TimestampMs::from_millis(at),
        kind: TaskEventKind::Known(kind),
    }
}

pub(crate) fn fold_events(events: &[anybot_protocol::TaskEvent]) -> TaskView {
    use std::collections::BTreeSet;
    let mut view = initial_view();
    let mut seen = BTreeSet::new();
    crate::task_replay::fold(&mut view, events, &mut seen);
    view
}
