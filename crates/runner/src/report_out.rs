use anybot_protocol::{FinishReason, KnownTaskEventKind, TaskEvent, TaskEventKind};
use anybot_trust::ReportDue;

use crate::error::TurnError;
use crate::runner::{TurnCtx, TurnRunner};

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
        let TaskEventKind::Known(KnownTaskEventKind::ModelAnswer { text, finish, .. }) = &row.kind
        else {
            continue;
        };
        if text == anybot_trust::CODE_TURN_ANSWER_EMPTY || *finish == FinishReason::Interrupted {
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
    let due = anybot_trust::report_due(&rows).map_err(TurnError::Report)?;
    if due == ReportDue::None {
        return Ok(());
    }
    let goal = goal_of(&rows);
    let reply = last_reply_of(&rows);
    let report = anybot_trust::delivery_report_for_task(&rows, &ctx.task, &goal, reply.as_deref())
        .map_err(TurnError::Report)?;
    runner
        .append_task_event(
            &ctx.task,
            KnownTaskEventKind::DeliveryReport {
                report: Box::new(report),
            },
        )
        .await
        .map(|_| ())
}
