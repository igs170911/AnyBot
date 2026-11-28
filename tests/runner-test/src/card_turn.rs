use std::sync::Arc;
use std::time::Duration;

use anybot_foundation::TaskId;
use anybot_protocol::{CardAnswer, KnownTaskEventKind, TaskEventKind};
use anybot_runner::{StoredBody, TurnError, TurnOutcome};
use test_harness::{Step, sleep_real, timeout_real};
use tokio::task::JoinHandle;

use crate::turn::{Device, TurnWorld};

pub type TurnJob = JoinHandle<Result<TurnOutcome, TurnError>>;

pub struct AskedTurn {
    pub task: TaskId,
    pub card: String,
    job: TurnJob,
}

impl AskedTurn {
    pub async fn answer(
        self,
        world: &TurnWorld,
        answer: CardAnswer,
    ) -> Result<TurnOutcome, String> {
        world.answer_shown_card(self.task, answer).await?;
        join_turn(self.job).await
    }

    pub async fn finish(self) -> Result<TurnOutcome, String> {
        join_turn(self.job).await
    }

    #[must_use]
    pub fn is_running(&self) -> bool {
        !self.job.is_finished()
    }
}

#[must_use]
pub fn shown_cards(world: &TurnWorld) -> Vec<(TaskId, String)> {
    let mut cards = Vec::new();
    for (row, _probe) in world.record.entries() {
        let StoredBody::Task { task, event, .. } = &row.body else {
            continue;
        };
        if let TaskEventKind::Known(KnownTaskEventKind::CardShown { card, .. }) = &event.kind {
            cards.push((*task, card.clone()));
        }
    }
    cards
}

#[must_use]
pub fn send_job(world: &Arc<TurnWorld>, from: Device, text: &str) -> TurnJob {
    let shared = Arc::clone(world);
    let text = text.to_owned();
    tokio::spawn(async move { shared.send(from, &text).await })
}

pub async fn join_turn(job: TurnJob) -> Result<TurnOutcome, String> {
    timeout_real(job, Duration::from_secs(10))
        .await
        .map_err(|e| format!("turn timed out: {e}"))?
        .map_err(|e| format!("join failed: {e}"))?
        .map_err(|e| format!("turn error: {e:?}"))
}

pub async fn send_to_card(
    world: &Arc<TurnWorld>,
    from: Device,
    text: &str,
) -> Result<AskedTurn, String> {
    let before = shown_cards(world).len();
    let job = send_job(world, from, text);
    wait_for_card_after(world, before, job).await
}

pub async fn script_to_card(world: &Arc<TurnWorld>, steps: Vec<Step>) -> Result<AskedTurn, String> {
    world.script(steps);
    send_to_card(world, Device::Phone, "go").await
}

async fn wait_for_card_after(
    world: &TurnWorld,
    before: usize,
    job: TurnJob,
) -> Result<AskedTurn, String> {
    for _step in 0..1000 {
        if let Some((task, card)) = shown_cards(world).get(before).cloned() {
            return Ok(AskedTurn { task, card, job });
        }
        if job.is_finished() {
            let outcome = join_turn(job).await?;
            return Err(format!("turn ended without a card: {:?}", outcome.reason));
        }
        sleep_real(Duration::from_millis(5)).await;
    }
    job.abort();
    Err("timed out waiting for a shown card".to_owned())
}
