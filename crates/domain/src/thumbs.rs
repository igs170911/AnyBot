use anybot_foundation::TimestampMs;
use anybot_protocol::{KnownTaskEventKind, TaskStatus};

use crate::task_view::TaskView;

pub(crate) fn note_thumbs(view: &mut TaskView, at_ms: TimestampMs, kind: &KnownTaskEventKind) {
    if !matches!(kind, KnownTaskEventKind::ThumbsUp { .. }) {
        return;
    }
    if view.status != TaskStatus::Completed {
        return;
    }
    view.report.user_confirmed = true;
    view.report.user_confirmed_at_ms = Some(at_ms.millis());
    view.upgrade_completed_detail();
}
