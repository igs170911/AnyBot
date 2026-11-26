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
    }
}

pub(crate) fn turns_ended() -> KnownTaskEventKind {
    KnownTaskEventKind::TurnEnded {
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
