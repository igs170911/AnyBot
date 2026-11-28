use anybot_foundation::TimestampMs;
use anybot_protocol::{KnownTaskEventKind, TaskStatus};

use crate::task_view::initial_view;
use crate::thumbs::note_thumbs;

fn at(ms: u64) -> TimestampMs {
    TimestampMs::from_millis(ms)
}

#[test]
fn thumbs_rule_only_counts_when_completed() {
    let mut view = initial_view();
    view.status = TaskStatus::Running;
    note_thumbs(
        &mut view,
        at(1_000),
        &KnownTaskEventKind::ThumbsUp { answer_seq: 9 },
    );
    assert!(!view.report.user_confirmed);
    assert_eq!(view.report.user_confirmed_at_ms, None);

    view.status = TaskStatus::Completed;
    note_thumbs(
        &mut view,
        at(2_000),
        &KnownTaskEventKind::ThumbsUp { answer_seq: 9 },
    );
    assert!(view.report.user_confirmed);
    assert_eq!(view.report.user_confirmed_at_ms, Some(2_000));

    note_thumbs(
        &mut view,
        at(3_000),
        &KnownTaskEventKind::ThumbsUpRemoved { answer_seq: 9 },
    );
    assert!(view.report.user_confirmed);

    view.status = TaskStatus::Cancelled;
    note_thumbs(
        &mut view,
        at(4_000),
        &KnownTaskEventKind::ThumbsUp { answer_seq: 9 },
    );
    assert_eq!(view.report.user_confirmed_at_ms, Some(2_000));
}
