use std::future::Future;
use std::pin::Pin;
use std::task::Poll;

use anybot_foundation::{TaskId, TimestampMs};
use anybot_protocol::{
    AnswerOrigin, CardAnswer, CardButton, KnownTaskEventKind, TaskEvent, TaskEventKind,
};

use crate::error::TurnError;
use crate::record::{Halt, Stored, StoredBody};
use crate::runner::{TurnCtx, TurnRunner};

pub(crate) const CARD_TIMEOUT_MS: u64 = 15 * 60 * 1_000;

pub type WatchFuture<'a> = Pin<Box<dyn Future<Output = ()> + Send + 'a>>;

pub trait TaskWatch: Send + Sync {
    fn changed(&self, task: TaskId) -> WatchFuture<'_>;
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SmartAnswer {
    pub(crate) due_ms: u64,
    pub(crate) answer: CardAnswer,
    pub(crate) seconds: u32,
}

pub(crate) struct CardWait<'a> {
    pub(crate) shown_seq: u64,
    pub(crate) buttons: &'a [CardButton],
    pub(crate) smart: Option<SmartAnswer>,
}

pub(crate) enum CardWaitOutcome {
    Answered(CardAnswer, AnswerOrigin),
    SmartAnswered(SmartAnswer),
    Halted(Halt),
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
    wait: &CardWait<'_>,
) -> Result<CardWaitOutcome, TurnError> {
    let unattended = ctx.initiator.is_unattended();
    let timeout_ms = runner
        .clock
        .now_ms()
        .millis()
        .saturating_add(CARD_TIMEOUT_MS);
    loop {
        let watch = runner.watch.changed(ctx.task);
        if let Some(found) = poll_wait_state(runner, ctx, wait).await? {
            return Ok(found);
        }
        let now_ms = runner.clock.now_ms().millis();
        if let Some(smart) = wait.smart
            && now_ms >= smart.due_ms
        {
            return Ok(CardWaitOutcome::SmartAnswered(smart));
        }
        if unattended && now_ms >= timeout_ms {
            return Ok(CardWaitOutcome::TimedOut);
        }
        let unattended_deadline = unattended.then_some(timeout_ms);
        let smart_deadline = wait.smart.map(|smart| smart.due_ms);
        match next_deadline(unattended_deadline, smart_deadline) {
            Some(at_ms) => {
                let sleep = runner.clock.sleep_until(TimestampMs::from_millis(at_ms));
                wait_watch_or_deadline(watch, sleep).await;
            }
            None => watch.await,
        }
    }
}

fn next_deadline(first: Option<u64>, second: Option<u64>) -> Option<u64> {
    match (first, second) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (one, None) | (None, one) => one,
    }
}

async fn poll_wait_state(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    wait: &CardWait<'_>,
) -> Result<Option<CardWaitOutcome>, TurnError> {
    let events = runner
        .record
        .task_events(ctx.task)
        .await
        .map_err(TurnError::Record)?;
    if let Some((answer, origin)) = answered_after(&events, wait) {
        return Ok(Some(CardWaitOutcome::Answered(answer, origin)));
    }
    if let Some(halt) = crate::halt::check_halt(runner, ctx).await? {
        return Ok(Some(CardWaitOutcome::Halted(halt)));
    }
    Ok(None)
}

fn answered_after(events: &[TaskEvent], wait: &CardWait<'_>) -> Option<(CardAnswer, AnswerOrigin)> {
    for event in events {
        if event.seq <= wait.shown_seq {
            continue;
        }
        if let TaskEventKind::Known(KnownTaskEventKind::CardAnswered { answer, origin }) =
            &event.kind
            && wait
                .buttons
                .contains(&crate::answer_effects::button_of(*answer))
        {
            return Some((*answer, *origin));
        }
    }
    None
}

async fn wait_watch_or_deadline(
    mut watch: WatchFuture<'_>,
    mut sleep: anybot_foundation::clock::PinBoxFuture<'static, ()>,
) {
    std::future::poll_fn(|cx| {
        if watch.as_mut().poll(cx).is_ready() || sleep.as_mut().poll(cx).is_ready() {
            return Poll::Ready(());
        }
        Poll::Pending
    })
    .await;
}
