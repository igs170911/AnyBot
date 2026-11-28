use anybot_foundation::{DeviceId, TaskId};
use anybot_protocol::{TurnEndReason, TurnInitiator};

use crate::error::TurnError;
use crate::runner::{TurnCtx, TurnOutcome, TurnRunner, UserMessage};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Steer {
    pub device: DeviceId,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SteerQueued {
    pub task: TaskId,
    pub turn: u32,
}

pub(crate) struct ActiveSteer {
    task: TaskId,
    turn: u32,
    own_turn: bool,
    queued: Vec<Steer>,
}

pub(crate) struct ClearOnDrop<'a>(pub(crate) &'a TurnRunner);

impl Drop for ClearOnDrop<'_> {
    fn drop(&mut self) {
        clear_turn(self.0);
    }
}

pub(crate) fn begin_turn(runner: &TurnRunner, task: TaskId, turn: u32, own_turn: bool) {
    if let Ok(mut guard) = runner.steer_slot.lock() {
        *guard = Some(ActiveSteer {
            task,
            turn,
            own_turn,
            queued: Vec::new(),
        });
    }
}

pub(crate) fn clear_turn(runner: &TurnRunner) {
    if let Ok(mut guard) = runner.steer_slot.lock() {
        *guard = None;
    }
}

pub(crate) fn enqueue(runner: &TurnRunner, steer: Steer) -> Result<SteerQueued, TurnError> {
    let Ok(mut guard) = runner.steer_slot.lock() else {
        return Err(TurnError::NoOwnTurn);
    };
    let Some(slot) = guard.as_mut() else {
        return Err(TurnError::NoOwnTurn);
    };
    if !slot.own_turn {
        return Err(TurnError::NoOwnTurn);
    }
    let queued = SteerQueued {
        task: slot.task,
        turn: slot.turn,
    };
    slot.queued.push(steer);
    Ok(queued)
}

pub(crate) async fn consume_for_step(runner: &TurnRunner, ctx: &TurnCtx) -> Result<(), TurnError> {
    let items = take_queued(runner);
    for item in items {
        runner
            .append_user_said(
                &ctx.task,
                &UserMessage {
                    device: item.device,
                    text: item.text,
                },
            )
            .await?;
    }
    Ok(())
}

fn take_queued(runner: &TurnRunner) -> Vec<Steer> {
    let Ok(mut guard) = runner.steer_slot.lock() else {
        return Vec::new();
    };
    let Some(slot) = guard.as_mut() else {
        return Vec::new();
    };
    std::mem::take(&mut slot.queued)
}

fn take_or_clear(runner: &TurnRunner) -> Vec<Steer> {
    let Ok(mut guard) = runner.steer_slot.lock() else {
        return Vec::new();
    };
    let Some(slot) = guard.as_mut() else {
        return Vec::new();
    };
    if !slot.queued.is_empty() {
        return std::mem::take(&mut slot.queued);
    }
    *guard = None;
    Vec::new()
}

pub(crate) fn drain_and_clear(runner: &TurnRunner) -> Vec<Steer> {
    let Ok(mut guard) = runner.steer_slot.lock() else {
        return Vec::new();
    };
    let items = guard
        .as_mut()
        .map_or_else(Vec::new, |slot| std::mem::take(&mut slot.queued));
    *guard = None;
    items
}

pub(crate) async fn finish_or_continue(
    runner: &TurnRunner,
    mut ctx: TurnCtx,
    mut reason: TurnEndReason,
) -> Result<TurnOutcome, TurnError> {
    let unread_steers = loop {
        crate::report_out::emit_report(runner, &ctx).await?;
        if reason != TurnEndReason::Answered {
            let unread: Vec<String> = drain_and_clear(runner)
                .into_iter()
                .map(|item| item.text)
                .collect();
            break unread;
        }
        let pending_steers = take_or_clear(runner);
        let Some(first) = pending_steers.first() else {
            break Vec::new();
        };
        let initiator = TurnInitiator::You {
            device: first.device,
        };
        for item in pending_steers {
            runner
                .append_user_said(
                    &ctx.task,
                    &UserMessage {
                        device: item.device,
                        text: item.text,
                    },
                )
                .await?;
        }
        ctx = crate::continue_turn::open_continued_ctx(runner, ctx.task, initiator).await?;
        reason = runner.run_turn_loop(&ctx).await?;
    };
    runner.save_last_ctx(&ctx);
    Ok(TurnOutcome {
        task: ctx.task,
        turn: ctx.turn,
        reason,
        pending: crate::approvals::take_parked(runner),
        unread_steers,
    })
}
