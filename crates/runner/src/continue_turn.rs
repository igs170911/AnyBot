use anybot_foundation::{TaskId, TurnId};
use anybot_protocol::{KnownTaskEventKind, TaskEvent, TaskEventKind, TaskStatus, TurnInitiator};

use crate::error::TurnError;
use crate::runner::{TurnCtx, TurnOutcome, TurnRunner, UserMessage};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinueTurn {
    pub task: TaskId,
    pub initiator: TurnInitiator,
    pub opening: Option<String>,
}

impl TurnRunner {
    pub async fn continue_task(&self, start: ContinueTurn) -> Result<TurnOutcome, TurnError> {
        let guard = self.lock_active()?;
        let outcome = self.run_continued(start).await;
        drop(guard);
        outcome
    }

    async fn run_continued(&self, start: ContinueTurn) -> Result<TurnOutcome, TurnError> {
        let existing = self
            .record
            .task_events(start.task)
            .await
            .map_err(TurnError::Record)?;
        if existing.is_empty() || !has_task_created(&existing) {
            return Err(TurnError::TaskNotFound { task: start.task });
        }
        let view = anybot_domain::replay(&existing).map_err(TurnError::Replay)?;
        if view.status == TaskStatus::Cancelled {
            return Err(TurnError::TaskClosed { task: start.task });
        }
        if let (Some(text), TurnInitiator::You { device }) = (start.opening, start.initiator) {
            self.append_user_said(&start.task, &UserMessage { device, text })
                .await?;
        }
        let turn = next_turn_number(&existing);
        let evidence_start = next_evidence_number(&existing);
        let ctx = TurnCtx {
            task: start.task,
            turn,
            turn_id: TurnId::generate(&*self.clock),
            evidence: tokio::sync::Mutex::new(evidence_start),
        };
        self.append_turn_event(&ctx, self.turn_started_kind(turn, start.initiator))
            .await?;
        let outcome = self.run_steps(&ctx, start.task).await?;
        self.save_last_ctx(&ctx);
        Ok(outcome)
    }
}

fn has_task_created(rows: &[TaskEvent]) -> bool {
    rows.iter().any(|row| {
        matches!(
            &row.kind,
            TaskEventKind::Known(KnownTaskEventKind::TaskCreated { .. })
        )
    })
}

fn next_turn_number(rows: &[TaskEvent]) -> u32 {
    let max_turn = rows
        .iter()
        .filter_map(|row| match &row.kind {
            TaskEventKind::Known(KnownTaskEventKind::TurnStarted { turn, .. }) => Some(*turn),
            TaskEventKind::Known(_) | TaskEventKind::Raw(_) => None,
        })
        .max();
    max_turn.map_or(1, |value| value.saturating_add(1))
}

fn next_evidence_number(rows: &[TaskEvent]) -> u32 {
    let max_short = rows
        .iter()
        .filter_map(|row| match &row.kind {
            TaskEventKind::Known(KnownTaskEventKind::EvidenceAdded { record }) => {
                Some(record.short.0)
            }
            TaskEventKind::Known(_) | TaskEventKind::Raw(_) => None,
        })
        .max();
    max_short.map_or(1, |value| value.saturating_add(1))
}

pub(crate) fn next_asked_step(rows: &[TaskEvent]) -> u32 {
    let max_step = rows
        .iter()
        .filter_map(|row| match &row.kind {
            TaskEventKind::Known(KnownTaskEventKind::ModelAsked { step }) => Some(*step),
            TaskEventKind::Known(_) | TaskEventKind::Raw(_) => None,
        })
        .max();
    max_step.map_or(1, |value| value.saturating_add(1))
}
