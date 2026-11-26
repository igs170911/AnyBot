use std::sync::Arc;

use anybot_foundation::TurnId;
use anybot_foundation::{BotId, Clock, DeviceId, EventId, RecordId, TaskId};
use anybot_messages::Lang;
use anybot_models::{Calibration, Capability, ModelRef, Session, ThinkingMode, ToolSpec};
use anybot_protocol::{
    ComputerRef, KnownTaskEventKind, TaskEventKind, TaskOpener, TurnEndReason, TurnInitiator,
};

use crate::error::TurnError;
use crate::record::{NewTaskEvent, RawStore, Record, RecordEntry, Stored, UserLine};
use crate::step::{StepFlow, run_step};

pub struct BotProfile {
    pub id: BotId,
    pub name: String,
    pub instructions: String,
    pub timezone: String,
}

pub struct TurnModel {
    pub primary: ModelRef,
    pub thinking: ThinkingMode,
    pub capability: Capability,
    pub calibration: Calibration,
    pub reserved_output: u64,
}

pub struct RunnerParts {
    pub gated: Arc<dyn anybot_trust::GatedComputer>,
    pub record: Arc<dyn Record>,
    pub raw: Arc<dyn RawStore>,
    pub session: Session,
    pub clock: Arc<dyn Clock>,
    pub bot: BotProfile,
    pub model: TurnModel,
    pub tools: Vec<ToolSpec>,
    pub lang: Option<Lang>,
    pub computer: ComputerRef,
}

pub struct TurnCtx {
    pub(crate) task: TaskId,
    pub(crate) turn: u32,
    pub(crate) turn_id: TurnId,
    evidence: tokio::sync::Mutex<u32>,
}

impl TurnCtx {
    #[must_use]
    pub(crate) fn peek_evidence(&self) -> u32 {
        let Ok(guard) = self.evidence.try_lock() else {
            return 1;
        };
        *guard
    }

    pub(crate) fn next_evidence(&self) -> u32 {
        let Ok(mut guard) = self.evidence.try_lock() else {
            return 1;
        };
        let value = *guard;
        *guard = value.saturating_add(1);
        value
    }
}

pub struct UserMessage {
    pub device: DeviceId,
    pub text: String,
}

pub struct TurnOutcome {
    pub task: TaskId,
    pub turn: u32,
    pub reason: TurnEndReason,
}

pub struct TurnRunner {
    pub(crate) gated: Arc<dyn anybot_trust::GatedComputer>,
    pub(crate) record: Arc<dyn Record>,
    pub(crate) raw: Arc<dyn RawStore>,
    pub(crate) session: Arc<tokio::sync::Mutex<Session>>,
    pub(crate) clock: Arc<dyn Clock>,
    pub(crate) bot: BotProfile,
    pub(crate) model: TurnModel,
    pub(crate) tools: Vec<ToolSpec>,
    pub(crate) lang: Lang,
    pub(crate) computer: ComputerRef,
    active: Arc<tokio::sync::Mutex<()>>,
}

impl TurnRunner {
    #[must_use]
    pub fn new(parts: RunnerParts) -> Self {
        let lang = crate::fixed_blocks::lang_of(parts.lang);
        TurnRunner {
            gated: parts.gated,
            record: parts.record,
            raw: parts.raw,
            session: Arc::new(tokio::sync::Mutex::new(parts.session)),
            clock: parts.clock,
            bot: parts.bot,
            model: parts.model,
            tools: parts.tools,
            lang,
            computer: parts.computer,
            active: Arc::new(tokio::sync::Mutex::new(())),
        }
    }

    pub async fn on_user_message(&self, message: UserMessage) -> Result<TurnOutcome, TurnError> {
        let Ok(guard) = Arc::clone(&self.active).try_lock_owned() else {
            return Err(TurnError::Busy);
        };
        let outcome = self.run_turn(message).await;
        drop(guard);
        outcome
    }

    async fn run_turn(&self, message: UserMessage) -> Result<TurnOutcome, TurnError> {
        let task = TaskId::generate(&*self.clock);
        self.append_user_said(&task, &message).await?;
        let goal = message.text.clone();
        self.append_task_event(
            &task,
            KnownTaskEventKind::TaskCreated {
                goal,
                opened_by: TaskOpener::You,
                bot: self.bot.id,
                parent: None,
            },
        )
        .await?;
        let turn = self
            .count_task_kind(&task, KindCount::TurnStarted)
            .await?
            .saturating_add(1);
        self.append_task_event(&task, self.turn_started_kind(turn, message.device))
            .await?;
        let ctx = TurnCtx {
            task,
            turn,
            turn_id: TurnId::generate(&*self.clock),
            evidence: tokio::sync::Mutex::new(1),
        };
        self.run_steps(&ctx, task).await
    }

    async fn run_steps(&self, ctx: &TurnCtx, task: TaskId) -> Result<TurnOutcome, TurnError> {
        loop {
            match run_step(self, ctx).await? {
                StepFlow::Continue => {}
                StepFlow::Ended(reason) => {
                    crate::report_out::emit_report(self, ctx).await?;
                    return Ok(TurnOutcome {
                        task,
                        turn: ctx.turn,
                        reason,
                    });
                }
            }
        }
    }

    pub(crate) async fn append_task_event(
        &self,
        task: &TaskId,
        kind: KnownTaskEventKind,
    ) -> Result<Stored, TurnError> {
        let new = NewTaskEvent {
            bot: self.bot.id,
            task: *task,
            turn: Some(1),
            event_id: EventId::generate(&*self.clock),
            at_ms: self.clock.now_ms(),
            kind: TaskEventKind::Known(kind),
        };
        self.record
            .append(RecordEntry::Task(new))
            .await
            .map_err(TurnError::Record)
    }

    async fn append_user_said(
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

    pub(crate) async fn count_task_kind(
        &self,
        task: &TaskId,
        which: KindCount,
    ) -> Result<u32, TurnError> {
        let rows = self
            .record
            .task_events(*task)
            .await
            .map_err(TurnError::Record)?;
        Ok(count_rows(&rows, which))
    }

    fn turn_started_kind(&self, turn: u32, device: DeviceId) -> KnownTaskEventKind {
        KnownTaskEventKind::TurnStarted {
            turn,
            initiator: TurnInitiator::You { device },
            computer: self.computer.clone(),
            model: crate::answer::model_name_of(&self.model.primary.endpoint),
            thinking: thinking_choice_of(self.model.thinking),
        }
    }

    pub(crate) async fn append_turn_end(
        &self,
        task: &TaskId,
        reason: TurnEndReason,
        reply_delivered: bool,
    ) -> Result<(), TurnError> {
        self.append_task_event(
            task,
            KnownTaskEventKind::TurnEnded {
                reason,
                reply_delivered,
            },
        )
        .await
        .map(|_| ())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KindCount {
    TurnStarted,
    ModelAsked,
}

fn count_rows(rows: &[anybot_protocol::TaskEvent], which: KindCount) -> u32 {
    let mut count: u32 = 0;
    for row in rows {
        let TaskEventKind::Known(kind) = &row.kind else {
            continue;
        };
        let matched = matches!(
            (which, kind),
            (
                KindCount::TurnStarted,
                KnownTaskEventKind::TurnStarted { .. }
            ) | (KindCount::ModelAsked, KnownTaskEventKind::ModelAsked { .. })
        );
        if matched {
            count = count.saturating_add(1);
        }
    }
    count
}

pub(crate) fn thinking_choice_of(mode: ThinkingMode) -> anybot_protocol::ThinkingChoice {
    match mode {
        ThinkingMode::Auto => anybot_protocol::ThinkingChoice::Auto,
        ThinkingMode::Fast => anybot_protocol::ThinkingChoice::Fast,
        ThinkingMode::Balanced => anybot_protocol::ThinkingChoice::Balanced,
        ThinkingMode::Deep => anybot_protocol::ThinkingChoice::Deep,
    }
}
