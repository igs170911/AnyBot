use anybot_foundation::{EventId, RecordId, TaskId};
use anybot_protocol::{KnownTaskEventKind, TaskEventKind};

use crate::error::TurnError;
use crate::record::{Appended, NewTaskEvent, RecordEntry, Stored, UserLine};
use crate::runner::{TurnCtx, TurnRunner, UserMessage};

impl TurnRunner {
    pub(crate) async fn append_budget_limits(&self, task: &TaskId) -> Result<(), TurnError> {
        for &(item, limit) in &self.bot.budget_overrides {
            self.append_task_event(task, KnownTaskEventKind::BudgetLimit { item, limit })
                .await?;
        }
        Ok(())
    }

    pub async fn append_task_event(
        &self,
        task: &TaskId,
        kind: KnownTaskEventKind,
    ) -> Result<Stored, TurnError> {
        self.append_task_event_in_turn(task, None, kind).await
    }

    pub(crate) async fn append_turn_event(
        &self,
        ctx: &TurnCtx,
        kind: KnownTaskEventKind,
    ) -> Result<Stored, TurnError> {
        self.append_task_event_in_turn(&ctx.task, Some(ctx.turn), kind)
            .await
    }

    pub(crate) async fn append_turn_unless_halted(
        &self,
        ctx: &TurnCtx,
        kind: KnownTaskEventKind,
    ) -> Result<Appended, TurnError> {
        let new = NewTaskEvent {
            bot: self.bot.id,
            task: ctx.task,
            turn: Some(ctx.turn),
            event_id: EventId::generate(&*self.clock),
            at_ms: self.clock.now_ms(),
            kind: TaskEventKind::Known(kind),
        };
        self.record
            .append_unless_halted(RecordEntry::Task(new), ctx.turn_started)
            .await
            .map_err(TurnError::Record)
    }

    pub(crate) async fn append_task_event_in_turn(
        &self,
        task: &TaskId,
        turn: Option<u32>,
        kind: KnownTaskEventKind,
    ) -> Result<Stored, TurnError> {
        let new = NewTaskEvent {
            bot: self.bot.id,
            task: *task,
            turn,
            event_id: EventId::generate(&*self.clock),
            at_ms: self.clock.now_ms(),
            kind: TaskEventKind::Known(kind),
        };
        self.record
            .append(RecordEntry::Task(new))
            .await
            .map_err(TurnError::Record)
    }

    pub(crate) async fn append_user_said(
        &self,
        task: &TaskId,
        message: &UserMessage,
    ) -> Result<(), TurnError> {
        let line = UserLine {
            record_id: RecordId::generate(&*self.clock),
            bot: self.bot.id,
            task: *task,
            device: message.device,
            text: message.text.clone(),
            at_ms: self.clock.now_ms(),
        };
        self.record
            .append(RecordEntry::UserSaid(line))
            .await
            .map(|_| ())
            .map_err(TurnError::Record)
    }
}
