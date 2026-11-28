use anybot_protocol::{KnownTaskEventKind, TaskEvent, TaskEventKind, TaskStatus};

use crate::error::TurnError;
use crate::runner::{TurnCtx, TurnRunner};

pub(crate) fn report_due(view: &anybot_domain::TaskView) -> bool {
    view.evidence.actions_taken
        && matches!(
            view.status,
            TaskStatus::Completed | TaskStatus::Unaccepted | TaskStatus::Failed
        )
}

fn goal_of(rows: &[TaskEvent]) -> String {
    for row in rows {
        let TaskEventKind::Known(KnownTaskEventKind::TaskCreated { goal, .. }) = &row.kind else {
            continue;
        };
        return goal.clone();
    }
    String::new()
}

fn last_reply_of(rows: &[TaskEvent]) -> Option<String> {
    let mut out: Option<String> = None;
    for row in rows {
        let TaskEventKind::Known(KnownTaskEventKind::ModelAnswer { text, .. }) = &row.kind else {
            continue;
        };
        if text == anybot_trust::CODE_TURN_ANSWER_EMPTY {
            out = None;
            continue;
        }
        out = Some(text.clone());
    }
    out
}

pub(crate) async fn emit_report(runner: &TurnRunner, ctx: &TurnCtx) -> Result<(), TurnError> {
    let rows = runner
        .record
        .task_events(ctx.task)
        .await
        .map_err(TurnError::Record)?;
    let view = anybot_domain::replay(&rows).map_err(TurnError::Replay)?;
    if !report_due(&view) {
        return Ok(());
    }
    let goal = goal_of(&rows);
    let reply = last_reply_of(&rows);
    let report =
        anybot_trust::delivery_report(&rows, &goal, reply.as_deref()).map_err(TurnError::Report)?;
    runner
        .append_turn_event(
            ctx,
            KnownTaskEventKind::DeliveryReport {
                report: Box::new(report),
            },
        )
        .await
        .map(|_| ())
}
