use anybot_foundation::TaskId;
use anybot_protocol::{FinishReason, KnownTaskEventKind, TaskEventKind, TurnEndReason};

use crate::error::TurnError;
use crate::record::{Halt, Stored, StoredBody};
use crate::runner::{TurnCtx, TurnRunner};

pub const CODE_TURN_REPLY_STOPPED: &str = "turn-reply-stopped";

#[must_use]
pub(crate) fn halt_end_reason(halt: Halt) -> TurnEndReason {
    match halt {
        Halt::Stopped => TurnEndReason::Stopped,
        Halt::Cancelled => TurnEndReason::Cancelled,
    }
}

#[must_use]
pub(crate) fn halt_in_rows(rows: &[Stored], task: &TaskId, turn_started: u64) -> Option<Halt> {
    let mut found: Option<Halt> = None;
    for row in rows {
        if row.stream_seq <= turn_started {
            continue;
        }
        let StoredBody::Task {
            task: owner, event, ..
        } = &row.body
        else {
            continue;
        };
        if owner != task {
            continue;
        }
        let TaskEventKind::Known(kind) = &event.kind else {
            continue;
        };
        match halt_kind(kind) {
            Some(Halt::Cancelled) => return Some(Halt::Cancelled),
            Some(Halt::Stopped) => found = Some(Halt::Stopped),
            None => {}
        }
    }
    found
}

fn halt_kind(kind: &KnownTaskEventKind) -> Option<Halt> {
    match kind {
        KnownTaskEventKind::TaskCancelled { .. } => Some(Halt::Cancelled),
        KnownTaskEventKind::TaskStopped => Some(Halt::Stopped),
        KnownTaskEventKind::TaskCreated { .. }
        | KnownTaskEventKind::DoneWhenChange { .. }
        | KnownTaskEventKind::DoneWhenAppliedByUser { .. }
        | KnownTaskEventKind::Wake { .. }
        | KnownTaskEventKind::TurnStarted { .. }
        | KnownTaskEventKind::TurnEnded { .. }
        | KnownTaskEventKind::ContextBuilt { .. }
        | KnownTaskEventKind::ModelAsked { .. }
        | KnownTaskEventKind::ModelAnswer { .. }
        | KnownTaskEventKind::ModelUsage { .. }
        | KnownTaskEventKind::ToolNotRun { .. }
        | KnownTaskEventKind::ModelSwapped { .. }
        | KnownTaskEventKind::GateResult { .. }
        | KnownTaskEventKind::ToolIntent { .. }
        | KnownTaskEventKind::ToolStarted { .. }
        | KnownTaskEventKind::ToolCompleted { .. }
        | KnownTaskEventKind::ToolFailed { .. }
        | KnownTaskEventKind::OutcomeUnknown { .. }
        | KnownTaskEventKind::Reconciled { .. }
        | KnownTaskEventKind::ActionVerified { .. }
        | KnownTaskEventKind::EvidenceAdded { .. }
        | KnownTaskEventKind::CriterionPassed { .. }
        | KnownTaskEventKind::CriterionFailed { .. }
        | KnownTaskEventKind::AcceptanceUnavailable
        | KnownTaskEventKind::VerificationStarted
        | KnownTaskEventKind::ClaimChecked { .. }
        | KnownTaskEventKind::BudgetUsage { .. }
        | KnownTaskEventKind::BudgetLimit { .. }
        | KnownTaskEventKind::CardShown { .. }
        | KnownTaskEventKind::CardAnswered { .. }
        | KnownTaskEventKind::QuestionShown { .. }
        | KnownTaskEventKind::QuestionAnswered { .. }
        | KnownTaskEventKind::DeviceReport { .. }
        | KnownTaskEventKind::DeliveryReport { .. }
        | KnownTaskEventKind::ModelUnreachable
        | KnownTaskEventKind::ComputerOffline
        | KnownTaskEventKind::GoalPaused { .. }
        | KnownTaskEventKind::ThumbsUp { .. }
        | KnownTaskEventKind::ThumbsUpRemoved { .. }
        | KnownTaskEventKind::Stalled { .. } => None,
    }
}

pub(crate) async fn check_halt(
    runner: &TurnRunner,
    ctx: &TurnCtx,
) -> Result<Option<Halt>, TurnError> {
    let rows = runner
        .record
        .conversation(runner.bot.id)
        .await
        .map_err(TurnError::Record)?;
    Ok(halt_in_rows(&rows, &ctx.task, ctx.turn_started))
}

pub(crate) async fn finish_halt(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    halt: Halt,
) -> Result<TurnEndReason, TurnError> {
    if halt == Halt::Stopped && !has_reply_text_in_turn(runner, ctx).await? {
        runner
            .append_turn_event(
                ctx,
                KnownTaskEventKind::ModelAnswer {
                    text: CODE_TURN_REPLY_STOPPED.to_owned(),
                    thinking: None,
                    tool_requests: Vec::new(),
                    finish: FinishReason::Interrupted,
                    native: None,
                },
            )
            .await?;
    }
    let reason = halt_end_reason(halt);
    runner.append_turn_end(ctx, reason.clone(), false).await?;
    Ok(reason)
}

async fn has_reply_text_in_turn(runner: &TurnRunner, ctx: &TurnCtx) -> Result<bool, TurnError> {
    let rows = runner
        .record
        .conversation(runner.bot.id)
        .await
        .map_err(TurnError::Record)?;
    Ok(rows.iter().any(|row| {
        let StoredBody::Task {
            task, turn, event, ..
        } = &row.body
        else {
            return false;
        };
        if *task != ctx.task || *turn != Some(ctx.turn) {
            return false;
        }
        let TaskEventKind::Known(KnownTaskEventKind::ModelAnswer { text, .. }) = &event.kind else {
            return false;
        };
        !text.is_empty() && text != anybot_trust::CODE_TURN_ANSWER_EMPTY
    }))
}
