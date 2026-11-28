use std::future::Future;
use std::pin::Pin;
use std::task::Poll;

use anybot_foundation::{TaskId, TimestampMs};
use anybot_protocol::{CardAnswer, KnownTaskEventKind, TaskEvent, TaskEventKind};

use crate::error::TurnError;
use crate::record::{Halt, Stored, StoredBody};
use crate::runner::{TurnCtx, TurnRunner};

pub(crate) const CARD_TIMEOUT_MS: u64 = 15 * 60 * 1_000;

pub type WatchFuture<'a> = Pin<Box<dyn Future<Output = ()> + Send + 'a>>;

pub trait TaskWatch: Send + Sync {
    fn changed(&self, task: TaskId) -> WatchFuture<'_>;
}

pub(crate) enum CardWaitOutcome {
    Answered(CardAnswer),
    Halted(Halt),
    TimedOut,
}

enum RaceOutcome {
    Changed,
    TimedOut,
}

pub(crate) fn task_seq_of(row: &Stored) -> u64 {
    match &row.body {
        StoredBody::Task { event, .. } => event.seq,
        StoredBody::UserSaid(_) => 0,
    }
}

pub(crate) async fn wait_for_card(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    shown_seq: u64,
) -> Result<CardWaitOutcome, TurnError> {
    let unattended = ctx.initiator.is_unattended();
    let deadline_ms = runner
        .clock
        .now_ms()
        .millis()
        .saturating_add(CARD_TIMEOUT_MS);
    let deadline = TimestampMs::from_millis(deadline_ms);
    loop {
        let watch = runner.watch.changed(ctx.task);
        if let Some(found) = poll_wait_state(runner, ctx, shown_seq).await? {
            return Ok(found);
        }
        if unattended {
            let sleep = runner.clock.sleep_until(deadline);
            if let RaceOutcome::TimedOut = wait_watch_or_deadline(watch, sleep).await {
                if let Some(found) = poll_wait_state(runner, ctx, shown_seq).await? {
                    return Ok(found);
                }
                return Ok(CardWaitOutcome::TimedOut);
            }
        } else {
            watch.await;
        }
    }
}

async fn poll_wait_state(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    shown_seq: u64,
) -> Result<Option<CardWaitOutcome>, TurnError> {
    let events = runner
        .record
        .task_events(ctx.task)
        .await
        .map_err(TurnError::Record)?;
    if let Some(answer) = answered_after(&events, shown_seq) {
        return Ok(Some(CardWaitOutcome::Answered(answer)));
    }
    if let Some(halt) = crate::halt::check_halt(runner, ctx).await? {
        return Ok(Some(CardWaitOutcome::Halted(halt)));
    }
    Ok(None)
}

fn answered_after(events: &[TaskEvent], shown_seq: u64) -> Option<CardAnswer> {
    for event in events {
        if event.seq <= shown_seq {
            continue;
        }
        if let TaskEventKind::Known(KnownTaskEventKind::CardAnswered { answer }) = &event.kind {
            return Some(*answer);
        }
    }
    None
}

async fn wait_watch_or_deadline(
    mut watch: WatchFuture<'_>,
    mut sleep: anybot_foundation::clock::PinBoxFuture<'static, ()>,
) -> RaceOutcome {
    std::future::poll_fn(|cx| {
        if watch.as_mut().poll(cx).is_ready() {
            return Poll::Ready(RaceOutcome::Changed);
        }
        if sleep.as_mut().poll(cx).is_ready() {
            return Poll::Ready(RaceOutcome::TimedOut);
        }
        Poll::Pending
    })
    .await
}
